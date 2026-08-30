# -*- coding: utf-8 -*-
# pyright: reportMissingImports=false, reportAttributeAccessIssue=false
"""ES ElementId remapping oracle runner (pyRevit / IronPython + CPython).

UNTESTED: written without a Revit install. Expect small Revit API surface
fixes on the first run inside Revit; every call is annotated with the API
member it relies on so those fixes are quick. Nothing in this module invents
on-disk byte layouts — it only asks the Revit API what it knows and saves
files for rvt-rs to study afterwards.

Program: docs/research/unified-research-report.md §15 (H-ES5).
Transition table mirrors research/es-remap/manifest.yaml.
Output contract: docs/schemas/es-observation.schema.json.

Fixture law honoured here:
  * one semantic mutation per transition (each transition starts from a fresh
    copy of the seed file, never from the previous transition's result)
  * every saved file is hashed (SHA-256) and referenced from its observations
  * observations carry ``oracle_agrees: null`` — this runner IS the oracle
    side; agreement is decided later when rvt-rs decodes the same files
"""

import json
import os
import time

import clr  # noqa: F401  (IronPython / pythonnet .NET bridge)

clr.AddReference("RevitAPI")
clr.AddReference("System")
clr.AddReference("System.Core")

from System import Guid, Int32, String  # noqa: E402
from System.Collections.Generic import Dictionary, IDictionary, IList, List  # noqa: E402
from System.IO import File  # noqa: E402
from System.Security.Cryptography import SHA256  # noqa: E402

from Autodesk.Revit.DB import (  # noqa: E402
    BuiltInCategory,
    Curve,
    CurveLoop,
    DirectShape,
    ElementId,
    ElementTransformUtils,
    CopyPasteOptions,
    FilteredElementCollector,
    GeometryCreationUtilities,
    GeometryObject,
    Level,
    Line,
    SaveAsOptions,
    Transaction,
    Transform,
    Wall,
    XYZ,
)
from Autodesk.Revit.DB.ExtensibleStorage import (  # noqa: E402
    AccessLevel,
    DataStorage,
    Entity,
    Schema,
    SchemaBuilder,
)

SCHEMA_VERSION = 1
FIXTURE_ID = "ES-remap-00"

# Stable GUIDs so Schema.Lookup finds the same definitions across sessions.
# (If a conflicting definition for one of these GUIDs is ever loaded in the
# same Revit process — the §15.12 schema-conflict suite — Schema.Lookup
# returns the loaded one; the runner records SchemaName to make that visible.)
S_ALL_GUID = "9c6a8b9e-5d4a-4a2e-9f0e-0e6d7a5b1c01"
S_CHILD_GUID = "9c6a8b9e-5d4a-4a2e-9f0e-0e6d7a5b1c02"
S_ROLE_GUID = "9c6a8b9e-5d4a-4a2e-9f0e-0e6d7a5b1c03"

FIELD_REF = "F_ref"
FIELD_LIST = "F_list"
FIELD_KEY_MAP = "F_key_map"
FIELD_VALUE_MAP = "F_value_map"
FIELD_CHILD = "F_child"
FIELD_CHILD_REF = "F_child_ref"
FIELD_NOTE = "F_note"
FIELD_ROLE = "role"

ROLES = ("W", "T", "X", "DS")

# transition id -> (kind per es-observation.schema.json, evidence tier)
# Tiers follow the unified report table: E1 = single-environment observation.
# Promotion beyond E1 is the coordinator's decision, never this script's.
TRANSITIONS = [
    ("N1", "noop_baseline", "E1"),
    ("N2", "noop_baseline", "E1"),
    ("N3", "noop_baseline", "E1"),
    ("N4", "noop_baseline", "E1"),
    ("R1", "scalar", "E1"),
    ("R2", "scalar", "E1"),
    ("C1", "copy", "E1"),
    ("C2", "copy", "E1"),
    ("C3a", "copy", "E1"),
    ("C4a", "copy", "E1"),
]


# --------------------------------------------------------------------------
# small helpers
# --------------------------------------------------------------------------


def eid_value(eid):
    """Revit 2024+ exposes ElementId.Value (Int64); earlier releases IntegerValue."""
    if eid is None:
        return None
    value = getattr(eid, "Value", None)
    if value is None:
        value = eid.IntegerValue
    value = int(value)
    return None if value < 0 else value  # InvalidElementId is -1


def sha256_file(path):
    with SHA256.Create() as hasher:
        data = File.ReadAllBytes(path)
        digest = hasher.ComputeHash(data)
    return "".join("{:02x}".format(b) for b in digest)


def write_json(path, payload):
    with open(path, "w") as fh:
        json.dump(payload, fh, indent=2, sort_keys=True)


def save_as(doc, path, compact=False):
    """Document.SaveAs(string, SaveAsOptions). Returns the SHA-256 of the file."""
    opts = SaveAsOptions()
    opts.OverwriteExistingFile = True
    if compact:
        opts.Compact = True
    doc.SaveAs(path, opts)
    return sha256_file(path)


def first_level(doc):
    level = FilteredElementCollector(doc).OfClass(Level).FirstElement()
    if level is None:
        raise RuntimeError("seed document has no Level; start from the default project template")
    return level


# --------------------------------------------------------------------------
# schemas
# --------------------------------------------------------------------------


def _lookup_or_build(guid_str, name, build):
    schema = Schema.Lookup(Guid(guid_str))
    if schema is not None:
        return schema
    builder = SchemaBuilder(Guid(guid_str))
    builder.SetSchemaName(name)
    builder.SetReadAccessLevel(AccessLevel.Public)
    builder.SetWriteAccessLevel(AccessLevel.Public)
    build(builder)
    return builder.Finish()


def child_schema():
    def build(b):
        b.AddSimpleField(FIELD_CHILD_REF, ElementId)

    return _lookup_or_build(S_CHILD_GUID, "RvtOracleSChild", build)


def all_schema():
    child = child_schema()

    def build(b):
        b.AddSimpleField(FIELD_REF, ElementId)
        b.AddArrayField(FIELD_LIST, ElementId)
        b.AddMapField(FIELD_KEY_MAP, ElementId, Int32)
        b.AddMapField(FIELD_VALUE_MAP, String, ElementId)
        nested = b.AddSimpleField(FIELD_CHILD, Entity)
        nested.SetSubSchemaGUID(child.GUID)
        b.AddSimpleField(FIELD_NOTE, String)

    return _lookup_or_build(S_ALL_GUID, "RvtOracleSAll", build)


def role_schema():
    def build(b):
        b.AddSimpleField(FIELD_ROLE, String)

    return _lookup_or_build(S_ROLE_GUID, "RvtOracleSRole", build)


# --------------------------------------------------------------------------
# seed construction (Phase A, S_All)
# --------------------------------------------------------------------------


def make_box(doc, origin, size=2.0, height=1.0):
    """DirectShape (Generic Model) box — an unhosted, level-independent target."""
    p0 = XYZ(origin[0], origin[1], 0.0)
    p1 = XYZ(origin[0] + size, origin[1], 0.0)
    p2 = XYZ(origin[0] + size, origin[1] + size, 0.0)
    p3 = XYZ(origin[0], origin[1] + size, 0.0)
    edges = List[Curve]()
    for a, b in ((p0, p1), (p1, p2), (p2, p3), (p3, p0)):
        edges.Add(Line.CreateBound(a, b))
    loop = CurveLoop.Create(edges)
    loops = List[CurveLoop]()
    loops.Add(loop)
    solid = GeometryCreationUtilities.CreateExtrusionGeometry(loops, XYZ.BasisZ, height)
    shape = DirectShape.CreateElement(doc, ElementId(BuiltInCategory.OST_GenericModel))
    geometry = List[GeometryObject]()
    geometry.Add(solid)
    shape.SetShape(geometry)
    return shape


def tag_role(element, role):
    entity = Entity(role_schema())
    entity.Set[String](FIELD_ROLE, role)
    element.SetEntity(entity)


def entity_for(target_id, control_id):
    """Entity(S_All) with every reference kind pointing at `target_id`."""
    schema = all_schema()
    entity = Entity(schema)
    entity.Set[ElementId](FIELD_REF, target_id)

    refs = List[ElementId]()
    refs.Add(target_id)
    refs.Add(control_id)
    entity.Set[IList[ElementId]](FIELD_LIST, refs)

    key_map = Dictionary[ElementId, Int32]()
    key_map[target_id] = 1001  # immutable marker identifies the entry after key remap
    key_map[control_id] = 1002
    entity.Set[IDictionary[ElementId, Int32]](FIELD_KEY_MAP, key_map)

    value_map = Dictionary[String, ElementId]()
    value_map["target"] = target_id
    value_map["control"] = control_id
    entity.Set[IDictionary[String, ElementId]](FIELD_VALUE_MAP, value_map)

    child = Entity(child_schema())
    child.Set[ElementId](FIELD_CHILD_REF, target_id)
    entity.Set[Entity](FIELD_CHILD, child)

    entity.Set[String](FIELD_NOTE, "control")
    return entity


def build_seed(doc):
    """Create W (wall), T and X (DirectShape boxes), DS (DataStorage); attach entities.

    Returns {role: ElementId}.
    """
    level = first_level(doc)
    t = Transaction(doc, "rvt-oracle: build ES-remap-00 seed")
    t.Start()
    try:
        wall = Wall.Create(doc, Line.CreateBound(XYZ(0, 0, 0), XYZ(10, 0, 0)), level.Id, False)
        target = make_box(doc, (20.0, 0.0))
        control = make_box(doc, (20.0, 10.0))
        storage = DataStorage.Create(doc)

        tag_role(wall, "W")
        tag_role(target, "T")
        tag_role(control, "X")
        tag_role(storage, "DS")

        wall.SetEntity(entity_for(target.Id, control.Id))
        storage.SetEntity(entity_for(target.Id, control.Id))
        t.Commit()
    except Exception:
        t.RollBack()
        raise
    return {"W": wall.Id, "T": target.Id, "X": control.Id, "DS": storage.Id}


# --------------------------------------------------------------------------
# API truth capture
# --------------------------------------------------------------------------


def elements_by_role(doc):
    """{role: [Element]} for every element carrying an S_Role entity."""
    schema = role_schema()
    found = {}
    for element in FilteredElementCollector(doc).WhereElementIsNotElementType():
        entity = element.GetEntity(schema)
        if entity is not None and entity.IsValid():
            role = entity.Get[String](FIELD_ROLE)
            found.setdefault(role, []).append(element)
    return found


def reference_occurrences(element):
    """Every ElementId leaf reachable in the element's S_All entity, with its path.

    Yields dicts: {"path": [...], "value": int|None}
    """
    schema = all_schema()
    entity = element.GetEntity(schema)
    if entity is None or not entity.IsValid():
        return []
    out = []
    out.append({"path": [{"kind": "field", "name": FIELD_REF}], "value": eid_value(entity.Get[ElementId](FIELD_REF))})
    for index, eid in enumerate(entity.Get[IList[ElementId]](FIELD_LIST)):
        out.append({"path": [{"kind": "field", "name": FIELD_LIST}, {"kind": "index", "index": index}], "value": eid_value(eid)})
    for pair in entity.Get[IDictionary[ElementId, Int32]](FIELD_KEY_MAP):
        out.append(
            {
                "path": [{"kind": "field", "name": FIELD_KEY_MAP}, {"kind": "map_key", "key": "marker:{}".format(int(pair.Value))}],
                "value": eid_value(pair.Key),
            }
        )
    for pair in entity.Get[IDictionary[String, ElementId]](FIELD_VALUE_MAP):
        out.append(
            {
                "path": [{"kind": "field", "name": FIELD_VALUE_MAP}, {"kind": "map_key", "key": str(pair.Key)}],
                "value": eid_value(pair.Value),
            }
        )
    child = entity.Get[Entity](FIELD_CHILD)
    if child is not None and child.IsValid():
        out.append(
            {
                "path": [{"kind": "field", "name": FIELD_CHILD}, {"kind": "field", "name": FIELD_CHILD_REF}],
                "value": eid_value(child.Get[ElementId](FIELD_CHILD_REF)),
            }
        )
    out.append({"path": [{"kind": "field", "name": FIELD_NOTE}], "value": None, "note": entity.Get[String](FIELD_NOTE)})
    return out


def capture_truth(doc, label):
    """API truth for one document state: roles, ids, unique ids, reference leaves."""
    roles = elements_by_role(doc)
    truth = {
        "schema_version": SCHEMA_VERSION,
        "label": label,
        "revit_version": int(doc.Application.VersionNumber),
        "document_title": doc.Title,
        "schemas": {
            "S_All": {"guid": S_ALL_GUID, "name": all_schema().SchemaName},
            "S_Child": {"guid": S_CHILD_GUID, "name": child_schema().SchemaName},
            "S_Role": {"guid": S_ROLE_GUID, "name": role_schema().SchemaName},
        },
        "elements": [],
    }
    for role in sorted(roles):
        for element in roles[role]:
            truth["elements"].append(
                {
                    "role": role,
                    "element_id": eid_value(element.Id),
                    "unique_id": element.UniqueId,
                    "category": element.Category.Name if element.Category is not None else None,
                    "references": reference_occurrences(element),
                }
            )
    return truth


# --------------------------------------------------------------------------
# transitions
# --------------------------------------------------------------------------


def set_scalar_ref(doc, element, new_id):
    schema = all_schema()
    t = Transaction(doc, "rvt-oracle: set F_ref")
    t.Start()
    try:
        entity = element.GetEntity(schema)
        entity.Set[ElementId](FIELD_REF, new_id)
        element.SetEntity(entity)
        t.Commit()
    except Exception:
        t.RollBack()
        raise


def copy_same_doc(doc, ids, offset=(0.0, 30.0, 0.0)):
    """ElementTransformUtils.CopyElements(Document, ICollection<ElementId>, XYZ) -> new ids."""
    collection = List[ElementId]()
    for eid in ids:
        collection.Add(eid)
    t = Transaction(doc, "rvt-oracle: copy (same document)")
    t.Start()
    try:
        created = ElementTransformUtils.CopyElements(doc, collection, XYZ(*offset))
        t.Commit()
    except Exception:
        t.RollBack()
        raise
    return [eid for eid in created]


def copy_cross_doc(src_doc, ids, dst_doc):
    """ElementTransformUtils.CopyElements(Document, ICollection<ElementId>, Document, Transform, CopyPasteOptions)."""
    collection = List[ElementId]()
    for eid in ids:
        collection.Add(eid)
    t = Transaction(dst_doc, "rvt-oracle: copy (cross document)")
    t.Start()
    try:
        created = ElementTransformUtils.CopyElements(src_doc, collection, dst_doc, Transform.Identity, CopyPasteOptions())
        t.Commit()
    except Exception:
        t.RollBack()
        raise
    return [eid for eid in created]


def regenerate(doc):
    t = Transaction(doc, "rvt-oracle: regenerate")
    t.Start()
    doc.Regenerate()
    t.Commit()


# --------------------------------------------------------------------------
# observation records (docs/schemas/es-observation.schema.json)
# --------------------------------------------------------------------------


def observation(fixture_id, transition_id, kind, tier, document_key, before, after, unique_id, path, extra):
    record = {
        "schema_version": SCHEMA_VERSION,
        "observation_id": "{}:{}:{}".format(fixture_id, transition_id, "/".join(str(seg.get("name", seg.get("index", seg.get("key", "")))) for seg in path)),
        "fixture_id": fixture_id,
        "transition_id": transition_id,
        "kind": kind,
        "evidence_tier": tier,
        "document_key": document_key,
        "before_element_id": before,
        "after_element_id": after,
        "unique_id": unique_id,
        "path": path,
        "span": None,
        "oracle_agrees": None,
        "non_claims": [
            "API-side observation only; no rvt-rs decode compared yet",
            "no ES on-disk layout is asserted",
        ],
        "notes": [],
    }
    record.update(extra)
    return record


def diff_references(before_truth, after_truth, role_map):
    """Pair reference leaves by (role, path) across two truth captures.

    role_map maps a role in `before` to the role name to look up in `after`
    (identity for in-place mutations; e.g. {"W": "W'"} for copies where the
    caller has re-tagged the copy).
    """
    def index(truth):
        table = {}
        for element in truth["elements"]:
            for ref in element["references"]:
                key = (element["role"], json.dumps(ref["path"], sort_keys=True))
                table[key] = (element, ref)
        return table

    before_index = index(before_truth)
    after_index = index(after_truth)
    pairs = []
    for (role, path_key), (element, ref) in sorted(before_index.items()):
        after_role = role_map.get(role, role)
        after_entry = after_index.get((after_role, path_key))
        pairs.append((element, ref, after_entry))
    return pairs


# --------------------------------------------------------------------------
# driver
# --------------------------------------------------------------------------


class Runner(object):
    def __init__(self, uiapp, out_dir):
        self.app = uiapp.Application
        self.uiapp = uiapp
        self.out_dir = out_dir
        self.files = {}  # label -> {"path", "sha256"}
        self.observations = []
        self.log_lines = []
        if not os.path.isdir(out_dir):
            os.makedirs(out_dir)

    # -- bookkeeping -------------------------------------------------------

    def log(self, message):
        line = "{} {}".format(time.strftime("%H:%M:%S"), message)
        self.log_lines.append(line)
        print(line)

    def path(self, label, ext=".rvt"):
        return os.path.join(self.out_dir, "{}{}".format(label, ext))

    def record_file(self, label, path):
        self.files[label] = {"path": path, "sha256": sha256_file(path)}
        return self.files[label]

    def document_key(self, label):
        return "{}:{}".format(FIXTURE_ID, self.files[label]["sha256"][:16])

    def truth_to_disk(self, label, truth):
        write_json(self.path("truth-" + label, ".json"), truth)
        return truth

    # -- seed -----------------------------------------------------------------

    def run_seed(self, active_doc):
        self.log("building seed in active document")
        roles = build_seed(active_doc)
        seed_path = self.path("seed-" + FIXTURE_ID)
        save_as(active_doc, seed_path)
        self.record_file("seed", seed_path)
        truth = self.truth_to_disk("seed", capture_truth(active_doc, "seed"))
        self.log("seed saved: {} ({})".format(seed_path, self.files["seed"]["sha256"][:12]))
        return roles, truth

    def open_seed_copy(self, label):
        """Fresh working copy of the seed for one transition (one mutation each)."""
        work = self.path("work-" + label)
        File.Copy(self.files["seed"]["path"], work, True)
        return self.app.OpenDocumentFile(work)

    # -- transitions ----------------------------------------------------------

    def emit_pairs(self, transition_id, kind, tier, label, before_truth, after_truth, role_map, extra):
        key = self.document_key(label)
        for element, ref, after_entry in diff_references(before_truth, after_truth, role_map):
            after_value = after_entry[1]["value"] if after_entry else None
            unique_id = after_entry[0]["unique_id"] if after_entry else element["unique_id"]
            payload = dict(extra)
            payload["revit_version"] = int(self.app.VersionNumber)
            payload["revit_build"] = self.app.VersionBuild
            payload["owner_role"] = element["role"]
            payload["file_before"] = self.files["seed"]
            payload["file_after"] = self.files[label]
            if after_entry is None:
                payload.setdefault("notes", []).append("reference occurrence absent after transition (owner or field missing)")
                payload["reference_transition"] = "EntityMissing"
            self.observations.append(
                observation(FIXTURE_ID, transition_id, kind, tier, key, ref["value"], after_value, unique_id, ref["path"], payload)
            )

    def run_noop(self, transition_id, tier, seed_truth):
        doc = self.open_seed_copy(transition_id)
        try:
            if transition_id == "N2":
                # open the N1 result, not the seed, so this measures a second no-op save
                doc.Close(False)
                doc = self.app.OpenDocumentFile(self.files["N1"]["path"])
            if transition_id == "N3":
                regenerate(doc)
            compact = transition_id == "N4"
            out = self.path(transition_id)
            save_as(doc, out, compact=compact)
            self.record_file(transition_id, out)
            truth = self.truth_to_disk(transition_id, capture_truth(doc, transition_id))
            self.emit_pairs(transition_id, "noop_baseline", tier, transition_id, seed_truth, truth, {}, {"operation": "Committed", "owner_transition": "Unchanged", "target_transition": "Unchanged", "mutation": transition_id})
        finally:
            doc.Close(False)

    def run_scalar(self, transition_id, tier, seed_truth):
        doc = self.open_seed_copy(transition_id)
        try:
            roles = elements_by_role(doc)
            wall = roles["W"][0]
            new_id = roles["X"][0].Id if transition_id == "R1" else ElementId.InvalidElementId
            set_scalar_ref(doc, wall, new_id)
            out = self.path(transition_id)
            save_as(doc, out)
            self.record_file(transition_id, out)
            truth = self.truth_to_disk(transition_id, capture_truth(doc, transition_id))
            self.emit_pairs(transition_id, "scalar", tier, transition_id, seed_truth, truth, {}, {"operation": "Committed", "owner_transition": "Unchanged", "target_transition": "Unchanged", "mutation": "set F_ref to X" if transition_id == "R1" else "set F_ref to InvalidElementId"})
        finally:
            doc.Close(False)

    def run_copy(self, transition_id, tier, seed_truth):
        doc = self.open_seed_copy(transition_id)
        dst = None
        try:
            roles = elements_by_role(doc)
            ids = [roles["W"][0].Id]
            if transition_id in ("C2", "C4a"):
                ids.append(roles["T"][0].Id)
            if transition_id in ("C1", "C2"):
                created = copy_same_doc(doc, ids)
                target_doc = doc
            else:
                # C3a / C4a: destination is a fresh project from the default template.
                # The seed's schemas are already in process memory (§15.10 caveat).
                dst = self.app.NewProjectDocument(self.app.DefaultProjectTemplate)
                created = copy_cross_doc(doc, ids, dst)
                target_doc = dst
            # Copies carry the S_Role entity with them; re-tag them so the truth
            # capture can tell copy from original (CopyCorrespondence::RoleMarker).
            t = Transaction(target_doc, "rvt-oracle: tag copies")
            t.Start()
            for eid in created:
                element = target_doc.GetElement(eid)
                entity = element.GetEntity(role_schema())
                if entity is not None and entity.IsValid():
                    tag_role(element, entity.Get[String](FIELD_ROLE) + "'")
            t.Commit()
            out = self.path(transition_id)
            save_as(target_doc, out)
            self.record_file(transition_id, out)
            if dst is not None:
                src_out = self.path(transition_id + "-source")
                save_as(doc, src_out)
                self.record_file(transition_id + "-source", src_out)
            truth = self.truth_to_disk(transition_id, capture_truth(target_doc, transition_id))
            role_map = {"W": "W'", "T": "T'" if transition_id in ("C2", "C4a") else "T"}
            self.emit_pairs(
                transition_id,
                "copy",
                tier,
                transition_id,
                seed_truth,
                truth,
                role_map,
                {
                    "operation": "Committed",
                    "owner_transition": "Copied",
                    "target_transition": "Copied" if transition_id in ("C2", "C4a") else "Unchanged",
                    "copy_set": [eid_value(eid) for eid in ids],
                    "created_ids": [eid_value(eid) for eid in created],
                    "cross_document": dst is not None,
                    "correspondence": "RoleMarker",
                },
            )
        finally:
            if dst is not None:
                dst.Close(False)
            doc.Close(False)

    # -- top level ------------------------------------------------------------

    def run_all(self, active_doc, only=None):
        roles, seed_truth = self.run_seed(active_doc)
        for transition_id, kind, tier in TRANSITIONS:
            if only and transition_id not in only:
                continue
            self.log("transition {} ({})".format(transition_id, kind))
            try:
                if kind == "noop_baseline":
                    self.run_noop(transition_id, tier, seed_truth)
                elif kind == "scalar":
                    self.run_scalar(transition_id, tier, seed_truth)
                elif kind == "copy":
                    self.run_copy(transition_id, tier, seed_truth)
            except Exception as err:  # keep going; the failure is itself evidence
                self.log("transition {} FAILED: {}".format(transition_id, err))
                self.observations.append(
                    observation(FIXTURE_ID, transition_id, kind, "E0", self.document_key("seed"), None, None, None, [{"kind": "opaque", "label": "transition failed"}], {"operation": "Rejected", "reason": str(err), "revit_version": int(self.app.VersionNumber), "revit_build": self.app.VersionBuild})
                )
        self.write_bundle(roles)

    def write_bundle(self, roles):
        write_json(self.path("observations", ".json"), self.observations)
        write_json(
            self.path("bundle", ".json"),
            {
                "schema_version": SCHEMA_VERSION,
                "fixture_id": FIXTURE_ID,
                "generated_at": time.strftime("%Y-%m-%dT%H:%M:%S"),
                "revit_version": int(self.app.VersionNumber),
                "revit_build": self.app.VersionBuild,
                "seed_roles": {role: eid_value(eid) for role, eid in roles.items()},
                "files": self.files,
                "observation_count": len(self.observations),
                "transitions": [t[0] for t in TRANSITIONS],
                "law": {"mutations_per_transition": 1, "discovery_source": "owned_synthetics", "production_corpora": "regression_only"},
            },
        )
        with open(self.path("runner", ".log"), "w") as fh:
            fh.write("\n".join(self.log_lines) + "\n")
        self.log("wrote {} observations to {}".format(len(self.observations), self.out_dir))

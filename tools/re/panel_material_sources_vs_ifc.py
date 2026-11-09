#!/usr/bin/env python3
"""Which record's material is the one Revit's export gives a curtain panel (RE-377, #1593).

Scores the output of `probes/examples/probe_re377_panel_material_sources.rs` against
Revit's own IFC export of the same project. The probe lists, for each curtain panel
of a Revit 2015 to 2023 save, every field of the panel's own record, of its type's
record and of the records its reference list names that holds a material's ElementId.
For each plate (`IfcPlate`, by `Tag`) with a material in Revit's export this counts,
for every source (role, class, field path and a digest of the other fields of the
parameter entry it sits in), how many plates

- have it,
- have it giving exactly Revit's material (`equal`),
- have it giving several materials of which Revit's is one (`contains`),
- have it giving other materials only (`different`).

Its `panel material source` lines are counts, fixed words, schema class and field
names and a digest, never a material name, an ElementId or a `Tag`; the `detail` lines
after them show the bytes behind a digest and are for the models whose results leave the
runner in full.

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).

Usage:

    python3 tools/re/panel_material_sources_vs_ifc.py <probe output> <revit.ifc>
"""

import collections
import hashlib
import json
import sys

import ifcopenshell
import ifcopenshell.util.element as element_util


def material_names(material):
    if material is None:
        return set()
    if material.is_a("IfcMaterial"):
        return {material.Name}
    if material.is_a("IfcMaterialLayerSet"):
        return {layer.Material.Name for layer in material.MaterialLayers if layer.Material}
    if material.is_a("IfcMaterialConstituentSet"):
        return {c.Material.Name for c in material.MaterialConstituents or [] if c.Material}
    if material.is_a("IfcMaterialProfileSet"):
        return {p.Material.Name for p in material.MaterialProfiles if p.Material}
    if material.is_a("IfcMaterialList"):
        return {m.Name for m in material.Materials}
    return set()


def revit_plates(path):
    """Tag -> the material names Revit's export gives each IfcPlate."""
    model = ifcopenshell.open(path)
    out = {}
    for plate in model.by_type("IfcPlate"):
        if plate.Tag:
            material = element_util.get_material(plate, should_skip_usage=True)
            out.setdefault(plate.Tag, material_names(material))
    return out


READ = {}


def read_probe(path):
    panels, records, summary = {}, {}, {}
    READ["materials"], READ["refs"] = set(), {}
    with open(path, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            if not line.startswith("{"):
                continue
            try:
                row = json.loads(line)
            except ValueError:
                continue
            kind = row.get("kind")
            if kind == "panel":
                panels[row["id"]] = row
            elif kind == "record":
                records[row["id"]] = row
            elif kind == "materials":
                READ["materials"] = set(row["names"])
            elif kind == "material_refs":
                READ["refs"][row["name"]] = row["by_class"]
            elif kind == "summary":
                summary = row
    return panels, records, summary


def digest(context):
    if not context:
        return "-"
    return hashlib.sha1(context.encode("utf-8")).hexdigest()[:8]


BYTES_BEFORE = {}


def sources_of(panel, records):
    """(role, class, path, context digest) -> the material names the panel's records give there.

    A byte hit, a material's `u32` ElementId found in a record's body whatever field it is in, is a
    source of path `bytes`, told apart by a digest of the six bytes before it.
    """
    out = collections.defaultdict(set)
    wanted = [(panel["id"], "instance")]
    wanted += [(t, "type") for t in panel["types"]]
    wanted += [(r, "ref") for r in panel["refs"]]
    for record_id, role in wanted:
        record = records.get(record_id)
        if not record:
            continue
        for source in record["sources"]:
            key = (role, record["class"], source["path"], digest(source["context"]))
            out[key].add(source["name"])
        for hit in record.get("byte_hits", []):
            before = " ".join(hit["before"].split()[-6:])
            BYTES_BEFORE[digest(before)] = before
            out[(role, record["class"], "bytes", digest(before))].add(hit["name"])
    return out


def main():
    panels, records, summary = read_probe(sys.argv[1])
    revit = revit_plates(sys.argv[2])
    scored = {
        tag: names for tag, names in revit.items() if names and tag.isdigit() and int(tag) in panels
    }
    print(
        "panel material sources: plates in Revit's export %d, with a material %d, "
        "panels the probe lists %d, plates scored %d"
        % (
            len(revit),
            sum(1 for names in revit.values() if names),
            len(panels),
            len(scored),
        )
    )
    print(
        "panel material sources: records selected %s, read %s, holding a material %s"
        % (
            summary.get("records_selected", "?"),
            summary.get("records_read", "?"),
            summary.get("records_with_a_material", "?"),
        )
    )
    table = collections.defaultdict(lambda: collections.Counter())
    any_equal = 0
    for tag, names in scored.items():
        found = sources_of(panels[int(tag)], records)
        if any(given == names for given in found.values()):
            any_equal += 1
        for key, given in found.items():
            counter = table[key]
            counter["present"] += 1
            if given == names:
                counter["equal"] += 1
            elif names <= given:
                counter["contains"] += 1
            else:
                counter["different"] += 1
    print(
        "panel material sources: plates with a source giving exactly Revit's material %d of %d"
        % (any_equal, len(scored))
    )
    targets = set().union(*scored.values()) if scored else set()
    print(
        "panel material references: Revit's plate materials %d, named in the file %d"
        % (len(targets), len(targets & READ["materials"]))
    )
    classes = collections.Counter()
    unreferenced = 0
    for name in targets & READ["materials"]:
        if name in READ["refs"]:
            classes.update(READ["refs"][name])
        else:
            unreferenced += 1
    print(
        "panel material references: of those named, %d held by no record's body" % unreferenced
    )
    for cls, count in sorted(classes.items(), key=lambda item: (-item[1], item[0])):
        print("panel material references: class %s holds one in %d records" % (cls, count))
    for key in sorted(table, key=lambda k: (-table[k]["equal"], -table[k]["contains"], k)):
        role, cls, path, ctx = key
        counter = table[key]
        print(
            "panel material source: %s %s %s %s present %d, equal %d, contains %d, different %d"
            % (
                role,
                cls,
                path,
                ctx,
                counter["present"],
                counter["equal"],
                counter["contains"],
                counter["different"],
            )
        )


    # The bytes behind each digest can be another ElementId of the model, so these lines are for the
    # models whose results leave the runner in full; the scrub of the others drops them.
    for key in sorted(BYTES_BEFORE):
        print("panel material source detail: bytes %s = %s" % (key, BYTES_BEFORE[key]))


if __name__ == "__main__":
    main()

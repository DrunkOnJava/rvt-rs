#!/usr/bin/env python3
"""Score what the rvt-rs MCP server tells an agent about a model (#1327).

`tools/re/mcp_client.py` starts the built `rvt-rs-mcp`, a scripted client with no language model,
and asks the fixed query set on the model's .rvt file: the summary, the levels, every element's
row, the totals per IFC entity, Revit class and level, the rooms with their areas, `get_element`
for the five lowest ElementIds of each IFC entity, the diagnostics, and an `export_ifc`. This
scores the answers twice.

Consistency. The server answers from the model the IFC exporter builds, so each answer must equal
the IFC export of the same run. Any difference is a bug in one of the two and fails the check
(exit status 1), counted per check and listed (up to 25 each). Checked, against the run's own
export (`rvt-rs.ifc`, or the server's own when Measure was given `ifc_flags`, which the server's
default decode does not take):

- the protocol: `server/discover` lists 2026-07-28, every tool has an output schema, the text and
  structured results agree, the server exits cleanly when its stdin closes;
- the summary equals `get_summary`, and its counts equal the rows, the filtered totals, the IFC's
  entities and the export diagnostics;
- the rows are the IFC's elements and spaces (`IfcElement` and `IfcSpace`) by GlobalId, in both
  directions, with the same entity, Name, LongName of a space, Tag, PredefinedType and storey (an
  absent name stands for the exporter's `<class>-<ElementId>` placeholder); the IfcOpeningElements
  the writer cuts for doors and windows are the only IFC elements a row may lack, and a GlobalId
  that two IFC elements share is a failure;
- the levels are the IFC's storeys, with their elevations and the elements each holds;
- every room is an IfcSpace whose Name is its number (or `<name>:<ElementId>` without one), with
  the same LongName, storey and outline source, and an area equal to that of its body's plan
  outline;
- `get_element` on the sampled elements: the property and quantity sets, the trust markers, the
  type object (entity, Name, Tag and sets), the placement (the translation, not the rotation), the
  host, and whether the body is none or mapped, each equal to the IFC's;
- `get_diagnostics` equals the `diagnostics.json` rvt-ifc wrote, and the server's export equals
  rvt-ifc's (the hash Measure writes to `ifc.sha256`), when no flag was given; a check that was not
  measured is named in the verdict.

What it can and cannot catch. The server and the IFC writer share the one `IfcModel` the decode
builds, so this finds what the server or the writer does to that model on the way out: a field the
server projects wrongly, an element or a set the writer drops or adds, a name or a type the writer
makes at write time, a run that does not repeat. It cannot find a decode error (a wrong value read
from the file, an element not read), which both sides then share; the oracle below is what measures
those.

Oracle. With Revit's own export (`revit-export.ifc`), the answers are scored against it with the
existing scorers' pairing and tolerances: the class sets of `class_sets_vs_ifc.py` (elements by
Tag, spaces by GlobalId), the storeys of `storeys_vs_ifc.py`, the rooms' areas of
`space_profiles_vs_ifc.py` (the plan area of Revit's top surface, within 0.1% and 1%) and, for the
sampled elements, the property values of `psets_vs_ifc.py` (a relative 1e-5) and the quantities of
`qtos_vs_ifc.py`. These are measurements, not failures.

The lines starting `mcp answers:` are counts only, so the Measure workflow keeps them for the
models it keeps only counts of (tools/ci/scrub-holdout-results.sh): `Report.line` takes words
written in this file with `W()` and, as values, only integers and IFC entity names, and raises on
anything else. The lines that follow with an element's id, name or value are listed under them and
are not kept for those. A run that crashes ends with no verdict line, which the run summary reports
as such.

Usage:

    python3 tools/re/mcp_answers_vs_ifc.py --server <rvt-rs-mcp> --rvt <model.rvt>
        [--diagnostics <diagnostics.json>] [--dump <file>] [--all-details]
        <rvt-rs.ifc> [<revit-export.ifc>]

`--dump` writes the answers as the deterministic semantic dump (mcp_client.dump), `--all-details`
asks `get_element` for every element instead of a sample. `RVT_IFC_FLAGS` is the flags the export
was made with. Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins), NumPy and
Shapely.
"""

import argparse
import collections
import json
import os
import re
import shutil
import sys
import tempfile

import ifcopenshell
import ifcopenshell.util.element
import ifcopenshell.util.placement
import ifcopenshell.util.unit

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import class_sets_vs_ifc as classes  # noqa: E402
import mcp_client as mcp  # noqa: E402
import psets_vs_ifc as psets  # noqa: E402
import qtos_vs_ifc as qtos  # noqa: E402
import storeys_vs_ifc as storeys  # noqa: E402

FEET = 0.3048
LIST_LIMIT = 25
# The writer prints a number with six decimals; qtos_vs_ifc.same's tolerance.
ABSOLUTE = 1e-6
RELATIVE = 1e-5
AREA_ABSOLUTE = 1e-5
ROOM_SET = "RvtElementRecordGeometry"
MARKER_SUFFIXES = ("Source", "Resolved", "Refusal")
# Entities no reader of the IFC lists as an element.
WRITER_OPENING = "IfcOpeningElement"


# What may leave the runner for a model kept as counts only is the `mcp answers:` lines
# (tools/ci/scrub-holdout-results.sh), so a line is made of words written in this file and of
# numbers and IFC entity names, and nothing else: `W` makes the words, `Report.line` takes the
# numbers and names as values and raises on any other, so a line that interpolates a room's name
# or an ElementId cannot be printed.
WORDS = re.compile(r"[A-Za-z0-9 _:.,;()=%/+{}-]+")
ENTITY = re.compile(r"Ifc[A-Z][A-Za-z0-9]*")


class Fixed(str):
    """Words written in this file, with `{}` where a number or an IFC entity name goes."""


def W(text):
    if not WORDS.fullmatch(text) or "{" in text.replace("{}", "") or "}" in text.replace("{}", ""):
        raise ValueError(f"not fixed words: {text!r}")
    return Fixed(text)


def is_entity(value):
    if not isinstance(value, str) or not ENTITY.fullmatch(value):
        return False
    for schema in ("IFC4", "IFC2X3"):
        try:
            ifcopenshell.ifcopenshell_wrapper.schema_by_name(schema).declaration_by_name(value)
            return True
        except Exception:
            continue
    return False


class Report:
    def __init__(self):
        self.differing = []
        self.not_measured = []

    def line(self, template, *values):
        if not isinstance(template, Fixed):
            raise TypeError("a count line is made of fixed words: write them with W()")
        if template.count("{}") != len(values):
            raise ValueError(f"{template!r} takes {template.count('{}')} values")
        for value in values:
            if isinstance(value, bool) or not (isinstance(value, int) or is_entity(value)):
                raise TypeError(f"a count line holds numbers and IFC entity names, not {value!r}")
        print("mcp answers: " + template.format(*values), flush=True)

    def check(self, name, compared, differences):
        if not isinstance(name, Fixed) or "{}" in name:
            raise TypeError("a check is named with fixed words")
        print(f"mcp answers: consistency {name}: {compared} compared, {len(differences)} differ", flush=True)
        if differences:
            self.differing.append(name)
        for text in sorted(differences)[:LIST_LIMIT]:
            print(f"  difference {name}: {text}")
        if len(differences) > LIST_LIMIT:
            print(f"  difference {name}: ... {len(differences) - LIST_LIMIT} more")

    def skipped(self, name, why):
        """A check that was not measured, named in the verdict."""
        if not isinstance(name, Fixed) or not isinstance(why, Fixed):
            raise TypeError("a check is named with fixed words")
        self.not_measured.append(name)
        print(f"mcp answers: consistency {name}: NOT MEASURED, {why}", flush=True)

    def verdict(self):
        if self.differing:
            text = "FAIL: " + ", ".join(self.differing)
        elif self.not_measured:
            text = "PASS: every check measured is equal, NOT MEASURED: " + ", ".join(self.not_measured)
        else:
            text = "PASS: every check equal"
        print(f"mcp answers: consistency {text}", flush=True)


def number(value):
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def close(a, b, absolute=ABSOLUTE):
    return abs(a - b) <= max(absolute, RELATIVE * max(abs(a), abs(b)))


def equal_values(answer, written):
    """An answer's value and the one the IFC holds, numbers within the writer's rounding."""
    if isinstance(answer, list) or isinstance(written, list):
        return (
            isinstance(answer, list)
            and isinstance(written, list)
            and len(answer) == len(written)
            and all(equal_values(a, b) for a, b in zip(answer, written))
        )
    if number(answer) and number(written):
        return close(answer, written)
    if answer is None:
        return written in (None, "UNKNOWN")
    return answer == written


# ---- the IFC side -------------------------------------------------------------------------------


def unwrap(value, metres):
    if value is None:
        return None
    raw = value.wrappedValue
    if isinstance(raw, (tuple, list)):
        raw = list(raw)
    if "LengthMeasure" in value.is_a() and number(raw):
        raw = raw * metres
    return raw


def written_property(prop, metres):
    """(name, IfcValue type in capitals or None, value) of a property of a set."""
    if prop.is_a("IfcPropertySingleValue"):
        value = prop.NominalValue
        return prop.Name, (value.is_a().upper() if value is not None else None), unwrap(value, metres)
    if prop.is_a("IfcPropertyEnumeratedValue"):
        return prop.Name, None, [unwrap(v, metres) for v in prop.EnumerationValues or ()]
    if prop.is_a("IfcPropertyListValue"):
        return prop.Name, None, [unwrap(v, metres) for v in prop.ListValues or ()]
    return prop.Name, prop.is_a().upper(), None


def written_quantity(quantity, scales):
    attribute, unit = qtos.KINDS.get(quantity.is_a(), (None, None))
    if attribute is None:
        return quantity.Name, quantity.is_a().upper(), None
    raw = getattr(quantity, attribute)
    scale = scales.get(unit) if unit else 1.0
    return quantity.Name, quantity.is_a().upper(), None if raw is None or scale is None else float(raw) * scale


def written_set(definition, metres, scales):
    """{name, kind, properties: [(name, type, value)]} of a property set or quantity set."""
    if definition.is_a("IfcElementQuantity"):
        items = [written_quantity(q, scales) for q in definition.Quantities or ()]
        return {"name": definition.Name, "kind": "quantities", "properties": items}
    if definition.is_a("IfcPropertySet"):
        items = [written_property(p, metres) for p in definition.HasProperties or ()]
        return {"name": definition.Name, "kind": "properties", "properties": items}
    return None


def object_sets(obj, metres, scales):
    out = []
    for rel in obj.IsDefinedBy or ():
        if rel.is_a("IfcRelDefinesByProperties"):
            found = written_set(rel.RelatingPropertyDefinition, metres, scales)
            if found:
                out.append(found)
    return out


def type_sets(type_object, metres, scales):
    out = []
    for definition in getattr(type_object, "HasPropertySets", None) or ():
        found = written_set(definition, metres, scales)
        if found:
            out.append(found)
    return out


class Written:
    """What an IFC file holds that the answers are about."""

    def __init__(self, path):
        self.file = ifcopenshell.open(path)
        f = self.file
        self.metres = ifcopenshell.util.unit.calculate_unit_scale(f)
        self.scales = qtos.scales(f)
        self.products = {}
        self.repeated = 0
        for product in list(f.by_type("IfcElement")) + list(f.by_type("IfcSpace")):
            if product.GlobalId in self.products:
                self.repeated += 1
                continue
            self.products[product.GlobalId] = product
        self.storeys = sorted(f.by_type("IfcBuildingStorey"), key=lambda s: s.id())
        self.container = self.containers()

    def containers(self):
        """Product id -> the storey entity it is in, or the other container's entity, by containment and aggregation."""
        f = self.file
        where = {}
        for rel in f.by_type("IfcRelContainedInSpatialStructure"):
            for element in rel.RelatedElements:
                where[element.id()] = rel.RelatingStructure
        for rel in f.by_type("IfcRelAggregates"):
            whole = rel.RelatingObject
            if whole.is_a("IfcBuildingStorey"):
                for part in rel.RelatedObjects:
                    where.setdefault(part.id(), whole)
        for _ in range(3):
            for rel in f.by_type("IfcRelAggregates"):
                if rel.RelatingObject.id() in where:
                    for part in rel.RelatedObjects:
                        where.setdefault(part.id(), where[rel.RelatingObject.id()])
        return where

    def storey_entity(self, product):
        """The storey a product is in, directly, by aggregation or through the space that contains it; None where it is in the building or nowhere."""
        container = self.container.get(product.id())
        if container is None and product.is_a("IfcOpeningElement"):
            # An opening is contained in nothing: it is on the storey of the element it cuts.
            for void in product.VoidsElements or ():
                container = self.container.get(void.RelatingBuildingElement.id())
        for _ in range(4):
            if container is None or container.is_a("IfcBuildingStorey"):
                return container
            if not container.is_a("IfcSpace"):
                return None
            container = self.container.get(container.id())
        return None

    def storey_name(self, product):
        storey = self.storey_entity(product)
        return storey.Name if storey is not None else None

    def profile_area(self, product):
        """The plan area of the product's extruded body's profile, square metres, or None."""
        if product.Representation is None:
            return None
        for representation in product.Representation.Representations or ():
            for item in representation.Items or ():
                if not item.is_a("IfcExtrudedAreaSolid"):
                    continue
                profile = item.SweptArea
                if profile.is_a("IfcArbitraryProfileDefWithVoids"):
                    area = ring_area(profile.OuterCurve) - sum(ring_area(v) for v in profile.InnerCurves or ())
                    return area * self.metres**2
                if profile.is_a("IfcArbitraryClosedProfileDef"):
                    return ring_area(profile.OuterCurve) * self.metres**2
        return None

    def placement(self, product):
        if product.ObjectPlacement is None:
            return None
        matrix = ifcopenshell.util.placement.get_local_placement(product.ObjectPlacement)
        return matrix

    def body_kinds(self, product):
        kinds = set()
        if product.Representation is not None:
            for representation in product.Representation.Representations or ():
                for item in representation.Items or ():
                    kinds.add(item.is_a())
        return kinds


def ring_area(curve):
    if curve is None or not curve.is_a("IfcPolyline"):
        return 0.0
    points = [tuple(p.Coordinates)[:2] for p in curve.Points]
    return (
        abs(sum(p[0] * q[1] - q[0] * p[1] for p, q in zip(points, points[1:] + points[:1]))) / 2.0
    )


# ---- the answers --------------------------------------------------------------------------------


def key_of_category(name):
    """The IFC entity name in the schema's case (IFCWALL -> IfcWall)."""
    schema = ifcopenshell.ifcopenshell_wrapper.schema_by_name("IFC4")
    try:
        return schema.declaration_by_name(name).name()
    except Exception:
        return name


def check_protocol(report, answers, status):
    differences = []
    if "2026-07-28" not in answers["discover_versions"]:
        differences.append("server/discover does not list 2026-07-28")
    if answers["tools_without_output_schema"]:
        differences.append(f"tools without an output schema: {answers['tools_without_output_schema']}")
    needed = {"open_model", "get_summary", "list_elements", "get_element", "list_levels", "list_rooms", "get_diagnostics", "export_ifc"}
    missing = needed - set(answers["tools"])
    if missing:
        differences.append(f"tools missing: {sorted(missing)}")
    if answers["tools"] != sorted(answers["tools"]):
        differences.append("tools/list is not sorted by name")
    if status != 0:
        differences.append(f"exit status {status} after stdin closed")
    report.check(W("protocol"), 5, differences)


def check_counts(report, answers, written, diagnostics):
    summary = answers["summary"]
    rows = answers["rows"]
    differences = []
    compared = 0

    def same(label, a, b):
        nonlocal compared
        compared += 1
        if a != b:
            differences.append(f"{label}: {a} vs {b}")

    same("get_summary equals open_model", answers["summary_again"], summary)
    elements = summary["elements"]
    same("rows listed", answers["rows_total"], len(rows))
    same("summary total", elements["total"], len(rows))
    by_category = collections.Counter(row["provenance"].get("category") for row in rows)
    same("summary by_ifc_type", elements["by_ifc_type"], dict(by_category))
    by_class = collections.Counter(row["provenance"].get("class") for row in rows if row["provenance"].get("class"))
    same("summary by_revit_class", elements["by_revit_class"], dict(by_class))
    for name, total in answers["totals"]["category"].items():
        same(f"list_elements category {name}", total, by_category[name])
    for name, total in answers["totals"]["class"].items():
        expected = sum(1 for row in rows if (row["provenance"].get("class") or "").lower() == name.lower())
        same(f"list_elements class {name}", total, expected)
    for name, total in answers["totals"]["level"].items():
        expected = sum(1 for row in rows if (row.get("level") or "").lower() == name.lower())
        same(f"list_elements level {name}", total, expected)
    same("room_count", summary["room_count"], len(answers["rooms"]))
    listed = {row["provenance"].get("global_id") for row in rows}
    written_counts = collections.Counter(
        product.is_a().upper() for gid, product in written.products.items() if gid in listed
    )
    for name in sorted(set(by_category) | set(written_counts)):
        same(f"IFC {key_of_category(name)} count", by_category[name], written_counts[name])
    if diagnostics is not None:
        exported = diagnostics["exported"]
        # The diagnostics list an opening element by type but do not count it as a building element (RE-84).
        openings = elements["by_ifc_type"].get("IFCOPENINGELEMENT", 0)
        # The diagnostics also count the pieces the export folds into another element's body, which
        # the summary lists apart (`folded_into_bodies`) and are not rows.
        folded = elements.get("folded_into_bodies", {})
        folded_elements = sum(count for entity, count in folded.items() if entity != "IFCOPENINGELEMENT")
        same(
            "diagnostics exported building_elements",
            elements["total"] - openings + folded_elements,
            exported["building_elements"],
        )
        counted = {
            entity: elements["by_ifc_type"].get(entity, 0) + folded.get(entity, 0)
            for entity in set(elements["by_ifc_type"]) | set(folded)
        }
        same("diagnostics exported by_ifc_type", counted, exported["by_ifc_type"])
        digest = summary["diagnostics"]
        same("summary bounding_box_bodies", digest["bounding_box_bodies"], exported["bounding_box_bodies"])
        same("summary bounding_box_spaces", digest["bounding_box_spaces"], exported["bounding_box_spaces"])
        same("summary skipped", {s["reason"]: s["count"] for s in digest["skipped"]}, {s["reason"]: s["count"] for s in diagnostics["skipped"]})
        same("summary confidence_level", digest["confidence_level"], diagnostics["confidence"]["level"])
    report.check(W("counts"), compared, differences)
    openings = by_category["IFCOPENINGELEMENT"]
    report.line(
        W("elements the export folds into the body of another element, not rows: {}"),
        sum(summary["elements"].get("folded_into_bodies", {}).values()),
    )
    report.line(W("rows that are opening elements, which the diagnostics do not count as building elements: {}"), openings)
    for name in sorted(by_category):
        report.line(W("{}: {} rows"), key_of_category(name), by_category[name])
    report.line(W("rows listed: {}"), len(rows))
    return by_category


def row_sources(rows):
    seen = {}
    for row in rows:
        gid = row["provenance"].get("global_id")
        if gid:
            seen.setdefault(gid, []).append(row)
    return seen


def check_rows(report, answers, written):
    rows = answers["rows"]
    found_in = []
    entity, names, long_names, ids, predefined, levels = [], [], [], [], [], []
    compared = 0
    by_gid = row_sources(rows)
    for row in rows:
        if not row["provenance"].get("global_id"):
            found_in.append(f"element {row.get('id')}: no GlobalId")
    for gid, found in by_gid.items():
        if len(found) > 1:
            found_in.append(f"GlobalId {gid} on {len(found)} rows")
    if written.repeated:
        found_in.append(f"{written.repeated} IFC elements or spaces repeat a GlobalId")
    cut = 0
    for gid, product in written.products.items():
        if gid in by_gid:
            continue
        if product.is_a(WRITER_OPENING) and (product.HasFillings or product.VoidsElements):
            cut += 1
            continue
        found_in.append(f"{product.is_a()} {gid} ({product.Name!r}) is in the IFC and in no row")
    for gid, found in by_gid.items():
        row = found[0]
        product = written.products.get(gid)
        if product is None:
            found_in.append(f"element {row.get('id')} {row['provenance'].get('category')} {gid}: not in the IFC")
            continue
        compared += 1
        label = f"element {row.get('id')} {product.is_a()} {gid}"
        category = row["provenance"].get("category")
        if product.is_a().upper() != category:
            entity.append(f"{label}: entity {category} vs {product.is_a().upper()}")
        if row.get("name") is None:
            # An absent name stands for the exporter's own `<Revit class>-<ElementId>`, which no source named.
            placeholder = r"[A-Za-z0-9]+-" + re.escape(row.get("id") or "unnamed")
            if not re.fullmatch(placeholder, product.Name or ""):
                names.append(f"{label}: no name in the answer and the IFC Name {product.Name!r} is not <class>-<ElementId>")
        elif (product.Name or "") != row["name"]:
            names.append(f"{label}: name {row['name']!r} vs {product.Name!r}")
        long_name = (product.LongName or None) if product.is_a("IfcSpace") else None
        if (row.get("long_name") or None) != long_name:
            long_names.append(f"{label}: long name {row.get('long_name')!r} vs {long_name!r}")
        if not product.is_a("IfcSpace"):
            tag = getattr(product, "Tag", None)
            if (tag or None) != (row.get("id") or None):
                ids.append(f"{label}: id {row.get('id')!r} vs Tag {tag!r}")
        if hasattr(product, "PredefinedType"):
            if (product.PredefinedType or None) != (row.get("predefined_type") or None):
                predefined.append(f"{label}: predefined type {row.get('predefined_type')!r} vs {product.PredefinedType!r}")
        elif row.get("predefined_type"):
            predefined.append(f"{label}: predefined type {row.get('predefined_type')!r} on an entity that declares none")
        if (row.get("level") or None) != written.storey_name(product):
            levels.append(f"{label}: level {row.get('level')!r} vs storey {written.storey_name(product)!r}")
    report.check(W("element membership"), len(rows), found_in)
    report.check(W("element entities"), compared, entity)
    report.check(W("element names"), compared, names)
    report.check(W("element long names"), compared, long_names)
    report.check(W("element ids"), compared, ids)
    report.check(W("element predefined types"), compared, predefined)
    report.check(W("element storeys"), compared, levels)
    report.line(W("IFC openings the writer cuts for doors and windows, in no row: {}"), cut)
    report.line(W("IFC elements and spaces: {}"), len(written.products))
    return by_gid


def check_levels(report, answers, written, by_gid):
    levels = answers["levels"]
    differences = []
    compared = 0
    if len(levels) != len(written.storeys):
        differences.append(f"{len(levels)} levels vs {len(written.storeys)} IfcBuildingStorey")
    summary_levels = answers["summary"]["levels"]
    if summary_levels != levels:
        differences.append("the summary's levels differ from list_levels")
    inside = collections.Counter()
    for gid in by_gid:
        product = written.products.get(gid)
        storey = written.storey_entity(product) if product is not None else None
        if storey is not None:
            inside[storey.id()] += 1
    for level, storey in zip(levels, written.storeys):
        compared += 1
        label = f"level {level['index']} {level['name']!r}"
        if level["name"] != (storey.Name or ""):
            differences.append(f"{label}: name vs {storey.Name!r}")
        elevation = (storey.Elevation or 0.0) * written.metres
        if not close(level["elevation_metres"], elevation):
            differences.append(f"{label}: elevation {level['elevation_metres']} m vs {elevation} m")
        if not close(level["elevation_feet"] * FEET, level["elevation_metres"]):
            differences.append(f"{label}: elevation in feet and metres disagree")
        if level["element_count"] != inside[storey.id()]:
            differences.append(f"{label}: {level['element_count']} elements vs {inside[storey.id()]} in the IFC")
    report.check(W("levels"), compared, differences)


def check_rooms(report, answers, written):
    differences = []
    compared = 0
    rooms = answers["rooms"]
    by_gid = {}
    for room in rooms:
        by_gid.setdefault(room["provenance"].get("global_id"), []).append(room)
    spaces = {gid: p for gid, p in written.products.items() if p.is_a("IfcSpace")}
    space_rows = {
        row["provenance"].get("global_id"): row
        for row in answers["rows"]
        if row["provenance"].get("category") == "IFCSPACE"
    }
    answered = set(space_rows)
    if set(by_gid) != answered:
        differences.append(f"list_rooms names {len(by_gid)} spaces, list_elements {len(answered)}")
    with_area = 0
    for gid, found in by_gid.items():
        room = found[0]
        product = spaces.get(gid)
        label = f"room {room.get('id')} {gid}"
        if product is None:
            differences.append(f"{label}: not an IfcSpace in the IFC")
            continue
        compared += 1
        row_sets = {s["name"]: dict((p[0], p[2]) for p in s["properties"]) for s in object_sets(product, written.metres, written.scales)}
        record = row_sets.get(ROOM_SET, {})
        # The IFC Name is the number, or `<room name>:<ElementId>` without one, or, where neither was
        # read, the name the model kept (a family instance written as a space keeps its
        # `Family:Type:ElementId`, which the row gives), or the exporter's own `<class>-<ElementId>`
        # placeholder, which the row leaves out.
        if room.get("number"):
            expected = room["number"]
        elif room.get("name"):
            expected = f"{room['name']}:{room.get('id')}"
        else:
            expected = (space_rows.get(gid) or {}).get("name")
        if expected is not None and (product.Name or "") != expected:
            differences.append(f"{label}: Name {product.Name!r} is not the expected {expected!r}")
        if expected is None and not re.fullmatch(r"[A-Za-z0-9]+-" + re.escape(room.get("id") or "unnamed"), product.Name or ""):
            differences.append(f"{label}: no number or name in the answer and the IFC Name {product.Name!r} is not <class>-<ElementId>")
        if (room.get("name") or None) != (product.LongName or None):
            differences.append(f"{label}: name {room.get('name')!r} vs LongName {product.LongName!r}")
        if (room.get("level") or None) != written.storey_name(product):
            differences.append(f"{label}: level {room.get('level')!r} vs storey {written.storey_name(product)!r}")
        source = room["provenance"].get("space_outline_source")
        if source != record.get("SpaceOutlineSource"):
            differences.append(f"{label}: outline source {source!r} vs {record.get('SpaceOutlineSource')!r}")
        area = room.get("area_square_metres")
        if area is not None:
            with_area += 1
            drawn = written.profile_area(product)
            if drawn is None:
                differences.append(f"{label}: area {area} m2 and no plan outline in the IFC")
            elif abs(area - drawn) > max(AREA_ABSOLUTE, RELATIVE * max(area, drawn)):
                differences.append(f"{label}: area {area} m2 vs the outline's {drawn} m2")
            if not close(room["area_square_feet"] * 0.09290304, area):
                differences.append(f"{label}: area in square feet and square metres disagree")
    report.check(W("rooms"), compared, differences)
    report.line(W("rooms: {}"), len(rooms))
    report.line(W("rooms with an area: {}"), with_area)
    report.line(W("rooms without an area: {}"), len(rooms) - with_area)


def compare_sets(label, answer_sets, written_sets, differences):
    """Property and quantity sets of an answer and of the IFC, by set name, then by property name."""
    mine = collections.defaultdict(list)
    theirs = collections.defaultdict(list)
    for item in answer_sets:
        mine[item["name"]].append(item)
    for item in written_sets:
        theirs[item["name"]].append(item)
    compared = 0
    for name in sorted(set(mine) | set(theirs)):
        if len(mine[name]) != len(theirs[name]):
            differences.append(f"{label}: set {name!r} {len(mine[name])} times in the answer, {len(theirs[name])} in the IFC")
            continue
        for answer, held in zip(mine[name], theirs[name]):
            compared += 1
            if answer["kind"] != held["kind"]:
                differences.append(f"{label}: set {name!r} is {answer['kind']} in the answer, {held['kind']} in the IFC")
                continue
            mine_props = collections.defaultdict(list)
            for prop in answer["properties"]:
                key = prop["quantity"] if answer["kind"] == "quantities" else prop.get("ifc_type")
                mine_props[prop["name"]].append((key, prop["value"]))
            their_props = collections.defaultdict(list)
            for prop_name, kind, value in held["properties"]:
                their_props[prop_name].append((kind, value))
            for prop_name in sorted(set(mine_props) | set(their_props)):
                a, b = mine_props[prop_name], their_props[prop_name]
                if len(a) != len(b):
                    differences.append(f"{label}: {name}.{prop_name} {len(a)} times in the answer, {len(b)} in the IFC")
                    continue
                for (kind_a, value_a), (kind_b, value_b) in zip(a, b):
                    if kind_a is not None and kind_a != kind_b:
                        differences.append(f"{label}: {name}.{prop_name} type {kind_a} vs {kind_b}")
                    elif not equal_values(value_a, value_b):
                        differences.append(f"{label}: {name}.{prop_name} {value_a!r} vs {value_b!r}")
    return compared


def check_details(report, answers, written):
    rows = answers["rows"]
    first = {}
    for row in rows:
        if row.get("id"):
            first.setdefault(row["id"], row)
    count_by_id = collections.Counter(row.get("id") for row in rows if row.get("id"))
    differences = []
    type_differences = []
    type_presence = []
    compared = 0
    typed = 0
    sets_compared = 0
    for element_id in sorted(answers["details"], key=lambda text: mcp.id_key({"id": text})):
        detail = answers["details"][element_id]
        row = first[element_id]
        product = written.products.get(row["provenance"].get("global_id"))
        label = f"element {element_id}"
        if product is None:
            differences.append(f"{label}: not in the IFC")
            continue
        compared += 1
        label = f"element {element_id} {product.is_a()}"
        provenance = dict(detail["provenance"])
        provenance.pop("value_sources", None)
        if (
            provenance != row["provenance"]
            or detail.get("name") != row.get("name")
            or detail.get("long_name") != row.get("long_name")
            or detail.get("predefined_type") != row.get("predefined_type")
        ):
            differences.append(f"{label}: get_element and list_elements disagree")
        expected_level = row.get("level")
        if (detail.get("level") or {}).get("name") != expected_level:
            differences.append(f"{label}: level {(detail.get('level') or {}).get('name')!r} vs the row's {expected_level!r}")
        if len(detail["other_entities"]) != count_by_id[element_id] - 1:
            differences.append(f"{label}: {len(detail['other_entities'])} other entities vs {count_by_id[element_id] - 1} further rows")
        sets_compared += compare_sets(
            label, detail["property_sets"], object_sets(product, written.metres, written.scales), differences
        )
        markers = {
            name: value
            for item in detail["property_sets"]
            if item["name"] == ROOM_SET
            for name, value in ((p["name"], p["value"]) for p in item["properties"])
            if name.endswith(MARKER_SUFFIXES)
        }
        if markers != detail["trust_markers"]:
            differences.append(f"{label}: trust markers are not the record set's ...Source, ...Resolved and ...Refusal properties")
        kind = None if product.is_a("IfcSpace") else ifcopenshell.util.element.get_type(product)
        if product.is_a("IfcSpace"):
            kind = next((rel.RelatingType for rel in getattr(product, "IsTypedBy", None) or ()), None)
        answered_type = detail.get("element_type")
        typed += 1
        if answered_type is None:
            if kind is not None:
                type_presence.append(
                    f"{label}: no type in the answer, {kind.is_a()} {getattr(kind, 'Tag', None)!r} in the IFC"
                )
        elif kind is None:
            type_presence.append(f"{label}: type {answered_type['element_id']} in the answer, none in the IFC")
        else:
            if answered_type.get("ifc_type") != kind.is_a().upper():
                type_presence.append(f"{label}: type entity {answered_type.get('ifc_type')!r} vs {kind.is_a().upper()!r}")
            if answered_type.get("name") != kind.Name:
                type_presence.append(f"{label}: type name {answered_type.get('name')!r} vs {kind.Name!r}")
            if getattr(kind, "Tag", None) != answered_type["element_id"]:
                type_presence.append(
                    f"{label}: type id {answered_type['element_id']!r} vs Tag {getattr(kind, 'Tag', None)!r}"
                )
            sets_compared += compare_sets(
                f"{label} type",
                answered_type["property_sets"],
                type_sets(kind, written.metres, written.scales),
                type_differences,
            )
        kinds = written.body_kinds(product)
        if (detail["body"] == "none") != (not kinds):
            differences.append(f"{label}: body {detail['body']!r} vs representation {sorted(kinds)}")
        if detail["body"] == "mapped" and "IfcMappedItem" not in kinds:
            differences.append(f"{label}: body mapped and no IfcMappedItem")
        matrix = written.placement(product)
        place = detail.get("location_feet")
        if place is not None:
            if matrix is None:
                differences.append(f"{label}: location and no placement in the IFC")
            else:
                at = [matrix[0][3] * written.metres, matrix[1][3] * written.metres, matrix[2][3] * written.metres]
                if not all(close(a * FEET, b) for a, b in zip(place, at)):
                    differences.append(f"{label}: location {place} ft vs placement {[round(v / FEET, 6) for v in at]} ft")
        host = None
        cut = [product] if product.is_a("IfcOpeningElement") else []
        cut += [rel.RelatingOpeningElement for rel in getattr(product, "FillsVoids", None) or ()]
        for opening in cut:
            for void in opening.VoidsElements or ():
                host = void.RelatingBuildingElement.Tag
        if (detail.get("host_id") or None) != (host or None):
            differences.append(f"{label}: host {detail.get('host_id')!r} vs {host!r}")
    report.check(W("details"), compared, differences)
    report.check(W("type objects"), typed, type_presence)
    report.check(W("type object sets"), typed, type_differences)
    report.line(W("get_element property and quantity sets compared: {}"), sets_compared)
    return first


def diff_json(label, a, b, out):
    if isinstance(a, dict) and isinstance(b, dict):
        for key in sorted(set(a) | set(b)):
            if key not in a:
                out.append(f"{label}.{key}: only in rvt-ifc's")
            elif key not in b:
                out.append(f"{label}.{key}: only in the server's")
            else:
                diff_json(f"{label}.{key}", a[key], b[key], out)
    elif isinstance(a, list) and isinstance(b, list):
        if len(a) != len(b):
            out.append(f"{label}: {len(a)} items vs {len(b)}")
        for i, (x, y) in enumerate(zip(a, b)):
            diff_json(f"{label}[{i}]", x, y, out)
    elif number(a) and number(b) and (isinstance(a, float) or isinstance(b, float)):
        # The file prints an f32 as its shortest digits; the server's JSON holds the f32 widened to f64.
        if abs(a - b) > 1e-6 * max(1.0, abs(a), abs(b)):
            out.append(f"{label}: {a!r} vs {b!r}")
    elif a != b:
        out.append(f"{label}: {a!r} vs {b!r}")


def check_diagnostics(report, answers, diagnostics):
    if diagnostics is None:
        report.skipped(W("diagnostics"), W("no diagnostics.json beside the export, or the export was made with flags"))
        return
    differences = []
    diff_json("diagnostics", answers["diagnostics"]["diagnostics"], diagnostics, differences)
    report.check(W("diagnostics"), 1, differences)


def check_export(report, answers, ifc_path, flags):
    if flags:
        report.skipped(W("export hash"), W("the export of this run was made with flags the server default does not take"))
        return
    mine = mcp.sha256_without_export_time(answers["export"]["path"])
    theirs = mcp.sha256_without_export_time(ifc_path)
    differences = [] if mine == theirs else [f"export_ifc {mine} vs rvt-ifc {theirs}"]
    report.check(W("export hash"), 1, differences)


# ---- the oracle ---------------------------------------------------------------------------------


def our_members(rows):
    """key -> (class, ...) of the answers, as class_sets_vs_ifc.members keys an export."""
    out = {}
    untagged = repeated = 0
    for row in rows:
        category = row["provenance"].get("category")
        entity = key_of_category(category)
        if entity in classes.LEFT_OUT:
            continue
        if category == "IFCSPACE":
            out[("GlobalId", row["provenance"]["global_id"])] = entity
            continue
        if not row.get("id"):
            untagged += 1
            continue
        key = ("Tag", row["id"])
        if key in out:
            repeated += 1
            continue
        out[key] = entity
    return out, untagged, repeated


def oracle_classes(report, answers, revit_file, written):
    revit, revit_untagged, revit_repeated = classes.members(revit_file)
    ours, untagged, repeated = our_members(answers["rows"])
    report.line(W("classes: Revit export, elements and spaces paired: {}"), len(revit))
    report.line(W("classes: Revit export, elements left out for no Tag: {}"), revit_untagged)
    report.line(W("classes: Revit export, elements left out for a repeated Tag: {}"), revit_repeated)
    report.line(W("classes: the answers, elements and spaces paired: {}"), len(ours))
    report.line(W("classes: the answers, rows left out for no Tag: {}"), untagged)
    report.line(W("classes: the answers, rows left out for a repeated Tag: {}"), repeated)
    table = {}
    for entity in sorted({r[0] for r in revit.values()}):
        keys = [k for k, r in revit.items() if r[0] == entity]
        same = sum(1 for k in keys if ours.get(k) == entity)
        other = sum(1 for k in keys if k in ours and ours[k] != entity)
        absent = sum(1 for k in keys if k not in ours)
        table[entity] = (len(keys), same, other, absent)
        report.line(W("class {}: revit {}, same {}, other {}, absent {}"), entity, len(keys), same, other, absent)
    extra = collections.Counter(entity for key, entity in ours.items() if key not in revit)
    for entity in sorted(extra):
        report.line(W("class extra {}: {}"), entity, extra[entity])
    # The same table from rvt-rs's own export must be the answers' table.
    own, _, _ = classes.members(written.file)
    own_classes = {key: record[0] for key, record in own.items()}
    differences = []
    for key in sorted(set(own_classes) | set(ours)):
        if own_classes.get(key) != ours.get(key):
            differences.append(f"{key[0]} {key[1]}: {ours.get(key)} in the answers, {own_classes.get(key)} in the export")
    report.check(W("class sets"), len(own_classes), differences)
    return revit


STOREY_LINES = {
    "same": W("storeys: rows on the same storey as in Revit: {}"),
    "no storey, Revit has a storey": W("storeys: rows on no storey where Revit has a storey: {}"),
    "no storey, Revit has another container": W("storeys: rows on no storey where Revit has another container: {}"),
    "a storey, Revit has another container": W("storeys: rows on a storey where Revit has another container: {}"),
    "different": W("storeys: rows on a different storey from Revit: {}"),
}


def oracle_storeys(report, answers, revit_file):
    theirs = storeys.storeys(revit_file)
    verdicts = collections.Counter()
    compared = 0
    for row in answers["rows"]:
        category = row["provenance"].get("category")
        if category in ("IFCOPENINGELEMENT", "IFCSPACE") or not row.get("id"):
            continue
        key = row["id"]
        if key not in theirs:
            continue
        compared += 1
        mine, revit = row.get("level") or "<IfcBuilding>", theirs[key]
        if mine == revit:
            verdicts["same"] += 1
        elif not storeys.on_storey(mine):
            verdicts["no storey, Revit has a storey" if storeys.on_storey(revit) else "no storey, Revit has another container"] += 1
        elif not storeys.on_storey(revit):
            verdicts["a storey, Revit has another container"] += 1
        else:
            verdicts["different"] += 1
    report.line(W("storeys: rows the answers and Revit export share: {}"), compared)
    for what, count in sorted(verdicts.items()):
        report.line(STOREY_LINES[what], count)


def oracle_rooms(report, answers, revit_path):
    import plan_profiles_vs_ifc as plan

    try:
        revit = plan.tops(revit_path, ["IfcSpace"])
    except Exception as error:
        report.line(W("rooms: NOT MEASURED, Revit spaces could not be meshed"))
        return
    rooms = {room["provenance"]["global_id"]: room for room in answers["rooms"]}
    paired = [(room, revit[gid]) for gid, room in rooms.items() if gid in revit]
    with_area = [(room, top) for room, top in paired if room.get("area_square_feet") is not None and top[0] > 0]
    within = lambda tolerance: sum(1 for room, top in with_area if abs(room["area_square_feet"] - top[0]) / top[0] <= tolerance)
    report.line(W("rooms: in the answers: {}"), len(rooms))
    report.line(W("rooms: Revit spaces with a body: {}"), len(revit))
    report.line(W("rooms: paired by GlobalId: {}"), len(paired))
    report.line(W("rooms: paired with an area in the answers: {}"), len(with_area))
    report.line(W("rooms: paired without an area in the answers: {}"), len(paired) - len(with_area))
    report.line(W("rooms: area within 0.1% of Revit: {}"), within(1e-3))
    report.line(W("rooms: area within 1% of Revit: {}"), within(1e-2))
    report.line(W("rooms: area more than 1% from Revit: {}"), len(with_area) - within(1e-2))


def revit_sets_by_tag(revit):
    """Tag -> (Revit's property sets of the element) for the elements class_sets pairs."""
    return {key[1]: record[4] for key, record in revit.items() if key[0] == "Tag"}


VALUE_LINES = {
    (what, kind): W(f"values: {what} values {kind}: {{}}")
    for what in ("property", "quantity")
    for kind in ("in the answers", "Revit export does not write", "equal to Revit", "different from Revit")
}
VALUE_LINES[("property or quantity", "Revit writes that the answers lack")] = W(
    "values: property or quantity values Revit writes that the answers lack: {}"
)


def oracle_values(report, answers, revit_file, revit):
    products = revit_sets_by_tag(revit)
    metres = ifcopenshell.util.unit.calculate_unit_scale(revit_file)
    radians = ifcopenshell.util.unit.calculate_unit_scale(revit_file, "PLANEANGLEUNIT")
    scales = qtos.scales(revit_file)
    counts = collections.Counter()
    elements = 0
    for element_id, detail in answers["details"].items():
        product = products.get(element_id)
        if product is None:
            continue
        elements += 1
        theirs = {}
        for item in object_sets(product, metres, scales):
            for name, kind, value in item["properties"]:
                theirs[(item["name"], name)] = (kind, value, item["kind"])
        seen = set()
        for item in detail["property_sets"]:
            standard = item["name"].startswith("Pset_") and item["kind"] == "properties"
            quantity = item["name"].startswith("Qto_") and item["kind"] == "quantities"
            if not (standard or quantity):
                continue
            what = "property" if standard else "quantity"
            for prop in item["properties"]:
                key = (item["name"], prop["name"])
                seen.add(key)
                counts[(what, "in the answers")] += 1
                held = theirs.get(key)
                if held is None:
                    counts[(what, "Revit export does not write")] += 1
                    continue
                kind = prop["ifc_type"] if standard else prop["quantity"]
                if standard:
                    same = (kind is None or kind.upper() == held[0]) and (
                        equal_values(prop["value"], held[1])
                        if isinstance(prop["value"], list)
                        else psets.same(("v", prop["value"]), ("v", held[1]))
                    )
                else:
                    same = kind.upper() == held[0] and qtos.same((kind, prop["value"]), (kind, held[1]))
                counts[(what, "equal to Revit" if same else "different from Revit")] += 1
        for (set_name, name), held in theirs.items():
            if (set_name, name) in seen:
                continue
            if (set_name.startswith("Pset_") and held[2] == "properties") or (set_name.startswith("Qto_") and held[2] == "quantities"):
                counts[("property or quantity", "Revit writes that the answers lack")] += 1
    report.line(W("values: sampled rows the answers and Revit export share: {}"), elements)
    for (what, kind), count in sorted(counts.items()):
        report.line(VALUE_LINES[(what, kind)], count)


# ---- main ---------------------------------------------------------------------------------------


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--server", required=True)
    parser.add_argument("--rvt", required=True)
    parser.add_argument("--diagnostics")
    parser.add_argument("--dump")
    parser.add_argument("--all-details", action="store_true")
    parser.add_argument("ours")
    parser.add_argument("revit", nargs="?")
    args = parser.parse_args(argv)
    report = Report()
    flags = os.environ.get("RVT_IFC_FLAGS", "").strip()
    if not os.path.exists(args.ours) or not os.path.exists(args.server):
        report.line(W("consistency NOT MEASURED: no export of this model or no server binary"))
        return 0
    timeout = float(os.environ.get("MEASURE_STEP_TIMEOUT", "2400")) - 120
    scratch = tempfile.mkdtemp(prefix="mcp-answers-", dir=os.environ.get("RUNNER_TEMP"))
    try:
        server = mcp.Server(args.server, output_dir=scratch, timeout=timeout)
        try:
            answers = mcp.gather(
                server,
                os.path.abspath(args.rvt),
                scratch,
                details="all" if args.all_details else "sample",
                progress=lambda text: print(f"  {text}", flush=True),
            )
            status = server.close()
        except mcp.McpError as error:
            server.kill()
            report.line(W("consistency NOT MEASURED: the server did not answer the query set"))
            print(f"  error: {str(error)[:300]}")
            return 1
        if args.dump:
            with open(args.dump, "w", encoding="utf-8") as handle:
                handle.write(mcp.dump(answers))
        basis = answers["export"]["path"] if flags else args.ours
        diagnostics = None
        if args.diagnostics and os.path.exists(args.diagnostics) and not flags:
            with open(args.diagnostics, encoding="utf-8") as handle:
                diagnostics = json.load(handle)
        report.line(
            W("consistency basis: the export of this run")
            if not flags
            else W("consistency basis: the server own export, since the export of this run was made with flags")
        )
        written = Written(basis)
        check_protocol(report, answers, status)
        check_counts(report, answers, written, diagnostics)
        by_gid = check_rows(report, answers, written)
        check_levels(report, answers, written, by_gid)
        check_rooms(report, answers, written)
        check_details(report, answers, written)
        check_diagnostics(report, answers, diagnostics)
        check_export(report, answers, args.ours, flags)
        if args.revit:
            try:
                revit_file = ifcopenshell.open(args.revit)
                revit = oracle_classes(report, answers, revit_file, written)
                oracle_storeys(report, answers, revit_file)
                oracle_rooms(report, answers, args.revit)
                oracle_values(report, answers, revit_file, revit)
            except Exception as error:
                report.line(W("oracle: NOT MEASURED, the export of Revit could not be read or scored"))
                print(f"  error: {str(error)[:300]}")
        else:
            report.line(W("oracle: NOT MEASURED, Revit gave no export of this model"))
        report.verdict()
    finally:
        shutil.rmtree(scratch, ignore_errors=True)
    return 1 if report.differing else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

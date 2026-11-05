#!/usr/bin/env python3
"""Check that the Areas of one project agree across its saves (#1261, #1194, RE-333).

Autodesk's `rac_advanced` sample is saved by Revit 2024, 2025, 2026 and 2027.
Revit's own export of its Areas exists for none of them, so the Areas of the
2025 to 2027 saves are not measured against any Revit export. What rvt-rs can
be held to is the 2024 save, which is read by the 2024 reader: the same project
must give the same Areas on every later save. This is that gate (Griffin,
2026-11-05, "Same-project gate"). For each save after the first it compares,
Area by Area, paired by GlobalId:

- GlobalId: every Area of the first save is in the other, and no other is;
- number (`IfcSpace.Name`) and name (`IfcSpace.LongName`);
- storey: the name of the `IfcBuildingStorey` the space is aggregated under
  (rvt-rs writes an `IfcSpace` under its storey with `IfcRelAggregates`, as
  Revit's export does); an Area with no storey is a failure;
- outline source: `RvtElementRecordGeometry.SpaceOutlineSource`, which must be
  the one its release gets, not merely an Area value: `area_plan_topology` on
  the 2024 save and `area_plan_topology_unchecked` on the 2025, 2026 and 2027
  saves (Griffin, 2026-11-05, "Mark 2025 to 2027 in the file"; the rule is
  `partition_area_records::area_release_unchecked`). A 2025 Area that reads
  `area_plan_topology`, and a 2024 Area that reads the unchecked value, are
  failures;
- body area: the plan area of its top surface, within 1e-6 relative;
- top surface: the area of the symmetric difference of the two top surfaces,
  in Revit's internal frame, within 1e-6 relative of the body area, so a
  translation, a rotation or a shear that keeps the area is a difference;
- height: its lowest to its highest vertex, within 1e-5 ft;
- lowest vertex: the height of its lowest vertex, within 1e-5 ft, so a raised
  base is a difference.

It exits 1, and prints why, when any of these holds, so a Measure run turns red
(and a pull request that does not pass it is not landed, by the landing rule):

- a save is missing, or cannot be read;
- a save does not have exactly 25 Areas, or exactly 25 with a body (the project
  has 25, and each has one, so a save that reads fewer or more than that, none
  included, is a failure and never a pass on nothing);
- an Area has no storey;
- any Area differs in any field, or is missing from or extra in a later save.

What it cannot catch: an error that the 2024 read shares with the later reads
(the same wrong region, number or height on all four saves), and an error in
how the saves are exported that is the same in all of them. It does not say
that any Area is Revit's.

An Area is an `IfcSpace` whose `RvtElementRecordGeometry.SpaceOutlineSource` is
one of the Area values (`area_boundary`, `area_boundary_joined`,
`area_plan_topology`, `area_plan_topology_unchecked`, or `none`, the Area that
has no body).

Usage:

    python3 tools/re/areas_across_saves.py 2024=<ifc> 2025=<ifc> 2026=<ifc> 2027=<ifc>

The first save is the one the others are compared with. The output is counts
and field names only, one `areas across saves:` line each. Needs IfcOpenShell
(the version tools/ci/requirements-ifc.txt pins), NumPy and Shapely.
"""

import os
import sys

SET = "RvtElementRecordGeometry"
AREA_VALUES = (
    "area_boundary",
    "area_boundary_joined",
    "area_plan_topology",
    "area_plan_topology_unchecked",
    "none",
)
CHECKED_SOURCE = "area_plan_topology"
UNCHECKED_SOURCE = "area_plan_topology_unchecked"
# The releases whose Areas are marked unchecked: `AREA_UNCHECKED_RELEASES` in
# src/research/partition_area_records.rs.
UNCHECKED_RELEASES = (2025, 2026, 2027)
EXPECTED_AREAS = 25
AREA_TOLERANCE = 1e-6
SURFACE_TOLERANCE = 1e-6
HEIGHT_TOLERANCE_FEET = 1e-5
FIELDS = (
    "GlobalId",
    "number",
    "name",
    "storey",
    "outline source",
    "body area",
    "top surface",
    "height",
    "lowest vertex",
)


def expected_source(label):
    """The outline source the Areas of the save labelled `label` (its release)
    must read: the unchecked one for 2025 to 2027, `area_plan_topology` for the
    rest. A label that is not a release gets `area_plan_topology`."""
    try:
        release = int(label)
    except ValueError:
        return CHECKED_SOURCE
    return UNCHECKED_SOURCE if release in UNCHECKED_RELEASES else CHECKED_SOURCE


def storey_of(space):
    """The name of the storey a space is aggregated under, or contained in;
    None where it has neither."""
    import ifcopenshell.util.element

    parent = ifcopenshell.util.element.get_aggregate(space)
    while parent is not None and not parent.is_a("IfcBuildingStorey"):
        parent = ifcopenshell.util.element.get_aggregate(parent)
    if parent is None:
        parent = ifcopenshell.util.element.get_container(space)
    return parent.Name if parent is not None and parent.is_a("IfcBuildingStorey") else None


def read_areas(path):
    """GlobalId -> {number, name, storey, source, area, surface, height, low} of
    each Area in an IFC. The last four are None where the Area has no body."""
    import ifcopenshell
    import ifcopenshell.util.element

    import plan_profiles_vs_ifc as plan

    extents = {}
    tops = plan.tops(path, ["IfcSpace"], extents)
    model = ifcopenshell.open(path)
    areas = {}
    for space in model.by_type("IfcSpace"):
        props = ifcopenshell.util.element.get_psets(space).get(SET, {})
        source = props.get("SpaceOutlineSource")
        if source not in AREA_VALUES:
            continue
        top = tops.get(space.GlobalId)
        low_high = extents.get(space.GlobalId)
        areas[space.GlobalId] = {
            "number": space.Name,
            "name": space.LongName,
            "storey": storey_of(space),
            "source": source,
            "area": top[0] if top is not None else None,
            "surface": top[3] if top is not None else None,
            "height": (low_high[1] - low_high[0]) if low_high is not None else None,
            "low": low_high[0] if low_high is not None else None,
        }
    return areas


def differs(field, a, b):
    """Whether one field of the same Area differs between two saves. A missing
    body is a difference in every field of the body, whether or not the other
    save has one: it is counted as a failure of its own as well."""
    if field in ("body area", "height", "lowest vertex"):
        if a is None or b is None:
            return True
        if field == "body area":
            return abs(a - b) > AREA_TOLERANCE * max(abs(a), abs(b), 1.0)
        return abs(a - b) > HEIGHT_TOLERANCE_FEET
    return a != b


def surface_gap(a, b):
    """The area of the symmetric difference of two top surfaces and the larger
    of their areas, or (None, None) where either has no body."""
    if a is None or b is None:
        return None, None
    return float(a.symmetric_difference(b).area), max(float(a.area), float(b.area))


def compare(reference, other, reference_source=CHECKED_SOURCE, other_source=CHECKED_SOURCE):
    """Counts of the Areas of `other` against `reference`: how many were
    compared (in both), how many are missing from `other` or extra in it, and
    for each field how many are equal and how many differ. The outline source
    of each save must be the one its release gets (`reference_source`,
    `other_source`), so 2024 against 2025 expects the pair `area_plan_topology`
    and `area_plan_topology_unchecked`."""
    common = sorted(set(reference) & set(other))
    equal = {field: 0 for field in FIELDS}
    different = {field: 0 for field in FIELDS}
    # Paired by GlobalId, so it is equal for every Area compared.
    equal["GlobalId"] = len(common)
    for gid in common:
        a, b = reference[gid], other[gid]
        gap, larger = surface_gap(a["surface"], b["surface"])
        values = {
            "number": (a["number"], b["number"]),
            "name": (a["name"], b["name"]),
            "storey": (a["storey"], b["storey"]),
            "outline source": (a["source"], b["source"]),
            "body area": (a["area"], b["area"]),
            "top surface": (gap, larger),
            "height": (a["height"], b["height"]),
            "lowest vertex": (a["low"], b["low"]),
        }
        for field, (x, y) in values.items():
            if field == "top surface":
                bad = gap is None or gap > SURFACE_TOLERANCE * max(larger, 1.0)
            elif field == "storey":
                bad = x is None or y is None or x != y
            elif field == "outline source":
                bad = x != reference_source or y != other_source
            else:
                bad = differs(field, x, y)
            if bad:
                different[field] += 1
            else:
                equal[field] += 1
    return {
        "compared": len(common),
        "missing": len(set(reference) - set(other)),
        "extra": len(set(other) - set(reference)),
        "equal": equal,
        "different": different,
    }


def judge(labels, saves):
    """The failures of the checks, as lines, and the report lines, for the saves
    read (label -> areas, or None where a save could not be read)."""
    failures, report = [], []
    for label in labels:
        areas = saves.get(label)
        if areas is None:
            failures.append(f"{label}: no IFC to read")
            report.append(f"areas across saves: {label}: not read")
            continue
        bodies = sum(a["area"] is not None for a in areas.values())
        report.append(f"areas across saves: {label}: {len(areas)} Areas, {bodies} with a body")
        if len(areas) != EXPECTED_AREAS:
            failures.append(f"{label}: {len(areas)} Areas, not {EXPECTED_AREAS}")
        if bodies != EXPECTED_AREAS:
            failures.append(f"{label}: {bodies} Areas with a body, not {EXPECTED_AREAS}")
        unplaced = sum(a["storey"] is None for a in areas.values())
        if unplaced:
            failures.append(f"{label}: {unplaced} Areas without a storey")
        wanted = expected_source(label)
        wrong = sum(a["source"] != wanted for a in areas.values())
        if wrong:
            failures.append(f"{label}: {wrong} Areas whose outline source is not {wanted}")
    first = labels[0]
    reference = saves.get(first)
    for label in labels[1:]:
        other = saves.get(label)
        if reference is None or other is None:
            continue
        counts = compare(reference, other, expected_source(first), expected_source(label))
        fields = ", ".join(
            f"{field} {counts['equal'][field]} equal {counts['different'][field]} different"
            for field in FIELDS
        )
        report.append(
            f"areas across saves: {label} against {first}: {counts['compared']} compared,"
            f" {fields}, missing from {label} {counts['missing']}, extra in {label} {counts['extra']}"
        )
        bad = sum(counts["different"].values())
        if bad or counts["missing"] or counts["extra"]:
            failures.append(
                f"{label} against {first}: {bad} differences,"
                f" {counts['missing']} missing, {counts['extra']} extra"
            )
    return failures, report


def main(argv):
    pairs = []
    for argument in argv[1:]:
        label, _, path = argument.partition("=")
        if not label or not path:
            print(__doc__, file=sys.stderr)
            return 2
        pairs.append((label, path))
    if len(pairs) < 2:
        print(__doc__, file=sys.stderr)
        return 2
    labels = [label for label, _ in pairs]
    saves = {}
    problems = []
    for label, path in pairs:
        if not os.path.isfile(path):
            problems.append(f"{label}: the IFC is missing")
            continue
        try:
            saves[label] = read_areas(path)
        except Exception as error:  # a save that cannot be read is a failure
            problems.append(f"{label}: cannot be read ({type(error).__name__})")
    failures, report = judge(labels, saves)
    for line in report:
        print(line)
    failures = problems + failures
    for line in failures:
        print(f"areas across saves: FAILED: {line}")
    if failures:
        return 1
    print(
        f"areas across saves: passed: {EXPECTED_AREAS} Areas, {EXPECTED_AREAS} with a body,"
        f" on each of {len(labels)} saves, every field equal"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

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
- storey: the name of the `IfcBuildingStorey` that contains it;
- body area: the plan area of its top surface, within 1e-6 relative;
- height: its lowest to its highest vertex, within 1e-5 ft.

An Area without a body must be without one on both saves.

It exits 1, and prints why, when any of these holds, so a Measure run turns red
and a failed check blocks landing:

- a save is missing, or cannot be read;
- a save has fewer than 25 Areas (the project has 25, so a save that reads
  fewer than that, none included, is a failure and never a pass on nothing);
- any Area differs in any field, or is missing from or extra in a later save.

What it cannot catch: an error that the 2024 read shares with the later reads
(the same wrong region, number or height on all four saves), and an error in
how the saves are exported that is the same in all of them. It does not say
that any Area is Revit's.

An Area is an `IfcSpace` whose `RvtElementRecordGeometry.SpaceOutlineSource` is
one of the Area values (`area_boundary`, `area_boundary_joined`,
`area_plan_topology`, or `none`, the Area that has no body).

Usage:

    python3 tools/re/areas_across_saves.py 2024=<ifc> 2025=<ifc> 2026=<ifc> 2027=<ifc>

The first save is the one the others are compared with. The output is counts
and field names only, one `areas across saves:` line each. Needs IfcOpenShell
(the version tools/ci/requirements-ifc.txt pins), NumPy and Shapely.
"""

import os
import sys

SET = "RvtElementRecordGeometry"
AREA_VALUES = ("area_boundary", "area_boundary_joined", "area_plan_topology", "none")
EXPECTED_AREAS = 25
AREA_TOLERANCE = 1e-6
HEIGHT_TOLERANCE_FEET = 1e-5
FIELDS = ("GlobalId", "number", "name", "storey", "body area", "height")


def read_areas(path):
    """GlobalId -> {number, name, storey, area, height} of each Area in an IFC.
    `area` and `height` are None where the Area has no body."""
    import ifcopenshell
    import ifcopenshell.util.element

    import plan_profiles_vs_ifc as plan

    extents = {}
    tops = plan.tops(path, ["IfcSpace"], extents)
    model = ifcopenshell.open(path)
    areas = {}
    for space in model.by_type("IfcSpace"):
        props = ifcopenshell.util.element.get_psets(space).get(SET, {})
        if props.get("SpaceOutlineSource") not in AREA_VALUES:
            continue
        container = ifcopenshell.util.element.get_container(space)
        top = tops.get(space.GlobalId)
        low_high = extents.get(space.GlobalId)
        areas[space.GlobalId] = {
            "number": space.Name,
            "name": space.LongName,
            "storey": getattr(container, "Name", None),
            "area": top[0] if top is not None else None,
            "height": (low_high[1] - low_high[0]) if low_high is not None else None,
        }
    return areas


def differs(field, a, b):
    """Whether one field of the same Area differs between two saves."""
    if field in ("body area", "height"):
        if a is None or b is None:
            return (a is None) != (b is None)
        if field == "body area":
            return abs(a - b) > AREA_TOLERANCE * max(abs(a), abs(b), 1.0)
        return abs(a - b) > HEIGHT_TOLERANCE_FEET
    return a != b


def compare(reference, other):
    """Counts of the Areas of `other` against `reference`: how many were
    compared (in both), how many are missing from `other` or extra in it, and
    for each field how many are equal and how many differ."""
    common = sorted(set(reference) & set(other))
    keys = {
        "number": "number",
        "name": "name",
        "storey": "storey",
        "body area": "area",
        "height": "height",
    }
    equal = {field: 0 for field in FIELDS}
    different = {field: 0 for field in FIELDS}
    # Paired by GlobalId, so it is equal for every Area compared.
    equal["GlobalId"] = len(common)
    for gid in common:
        for field, key in keys.items():
            if differs(field, reference[gid][key], other[gid][key]):
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
        if len(areas) < EXPECTED_AREAS:
            failures.append(f"{label}: {len(areas)} Areas, fewer than {EXPECTED_AREAS}")
    first = labels[0]
    reference = saves.get(first)
    for label in labels[1:]:
        other = saves.get(label)
        if reference is None or other is None:
            continue
        counts = compare(reference, other)
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
    areas = len(saves[labels[0]])
    print(
        f"areas across saves: passed: {areas} Areas on each of {len(labels)} saves,"
        f" every field equal"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

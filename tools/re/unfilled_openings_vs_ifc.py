#!/usr/bin/env python3
"""Score the openings nothing fills in an rvt-rs IFC export against Revit's
own IFC4 export (RE-321, #1165).

Revit writes, besides the opening a door or window fills, an opening for each
element that cuts a host without filling it: a curtain wall embedded in a
wall, a fire hose cabinet, a rectangular wall opening, a door's further cut in
a second wall, and the openings of a floor's sketch voids, a roof's hatches
and a wall's own profile. Each is keyed by its Tag (the cutting element's
ElementId, the host's for a host's own geometry) and the Tag of the element
it voids, which two openings never share. Both files are meshed by
IfcOpenShell and moved into Revit's internal feet through each export's own
site placement.

By kind (the class of the element the Tag names in Revit's export, `none`
where it names none, and the class of the host) it reports how many openings
Revit writes, how many rvt-rs writes, and of those in both how many agree on
the box (within 0.01 ft), the GlobalId, the Name, the PredefinedType, the
`Reference` of `Pset_OpeningElementCommon` and
`Pset_EnvironmentalImpactIndicators`, and the `Depth` quantity; then the
openings rvt-rs writes that Revit's export does not, by kind.

Usage:

    python3 tools/re/unfilled_openings_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins) and NumPy.
"""

import collections
import os
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element
import ifcopenshell.util.unit
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ifc_common import site_frame  # noqa: E402

BOX_TOLERANCE_FEET = 0.01


def unfilled(path):
    """(Tag, host Tag) -> the opening's facts, for every opening nothing fills."""
    f = ifcopenshell.open(path)
    frame = site_frame(f)
    scale = ifcopenshell.util.unit.calculate_unit_scale(f)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    cutters = {}
    for element in f.by_type("IfcElement"):
        if not element.is_a("IfcOpeningElement") and element.Tag and element.Tag not in cutters:
            cutters[element.Tag] = element.is_a()
    out = {}
    for rel in f.by_type("IfcRelVoidsElement"):
        opening = rel.RelatedOpeningElement
        host = rel.RelatingBuildingElement
        if opening.HasFillings or not opening.Tag:
            continue
        box = None
        try:
            shape = ifcopenshell.geom.create_shape(settings, opening)
            verts = np.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
            if len(verts):
                moved = frame(verts)
                box = (moved.min(axis=0), moved.max(axis=0))
        except Exception:
            pass
        psets = ifcopenshell.util.element.get_psets(opening)
        common = psets.get("Pset_OpeningElementCommon", {})
        impact = psets.get("Pset_EnvironmentalImpactIndicators", {})
        depth = psets.get("Qto_OpeningElementBaseQuantities", {}).get("Depth")
        out[(opening.Tag, host.Tag)] = {
            "guid": opening.GlobalId,
            "name": opening.Name,
            "predefined": opening.PredefinedType,
            "box": box,
            "host": host.is_a(),
            "cutter": cutters.get(opening.Tag, "none"),
            "common": common.get("Reference"),
            "impact": impact.get("Reference"),
            "depth": None if depth is None else depth * scale,
        }
    return out


def main(argv):
    args = [a for a in argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    ours, revit = unfilled(args[0]), unfilled(args[1])
    both = [key for key in revit if key in ours]
    print(f"unfilled openings in Revit's export: {len(revit)}; in rvt-rs's: {len(ours)}; both: {len(both)}")
    rows = collections.defaultdict(collections.Counter)
    worst = []
    for key, theirs in revit.items():
        kind = f"{theirs['cutter']} in {theirs['host']}"
        row = rows[kind]
        row["revit"] += 1
        mine = ours.get(key)
        if mine is None:
            continue
        row["written"] += 1
        if theirs["box"] is not None and mine["box"] is not None:
            diff = float(
                max(
                    np.abs(mine["box"][0] - theirs["box"][0]).max(),
                    np.abs(mine["box"][1] - theirs["box"][1]).max(),
                )
            )
            row["box compared"] += 1
            if diff <= BOX_TOLERANCE_FEET:
                row["box within 0.01 ft"] += 1
            else:
                worst.append((diff, key, kind))
        for label, field in (
            ("GlobalId equal", "guid"),
            ("Name equal", "name"),
            ("PredefinedType equal", "predefined"),
            ("Pset_OpeningElementCommon.Reference equal", "common"),
            ("Pset_EnvironmentalImpactIndicators.Reference equal", "impact"),
        ):
            if mine[field] == theirs[field]:
                row[label] += 1
        if theirs["depth"] is not None:
            row["Depth in Revit's"] += 1
            if mine["depth"] is not None and abs(mine["depth"] - theirs["depth"]) <= 1e-5 * max(1.0, theirs["depth"]):
                row["Depth equal"] += 1
    for kind, row in sorted(rows.items()):
        print(f"{kind}:")
        for label, count in row.items():
            print(f"  {label}: {count}")
    extra = collections.Counter(
        f"{mine['cutter']} in {mine['host']}" for key, mine in ours.items() if key not in revit
    )
    print(f"written that Revit's export does not: {sum(extra.values())}")
    for kind, count in sorted(extra.items()):
        print(f"  {kind}: {count}")
    for diff, key, kind in sorted(worst, reverse=True)[:60]:
        print(f"  box differs by {diff:.3f} ft: {kind}, Tag {key[0]} in {key[1]}")
    for key, mine in sorted(ours.items()):
        if key not in revit:
            print(f"  extra: {mine['cutter']} in {mine['host']}, Tag {key[0]} in {key[1]}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

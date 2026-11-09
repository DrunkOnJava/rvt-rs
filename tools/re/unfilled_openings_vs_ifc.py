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
openings rvt-rs writes that Revit's export does not, by kind, apart from those
that a sibling file's export (another Revit export in the same directory with
the same first four name words) holds, which are counted on their own line.

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
from ifc_common import siblings, site_frame  # noqa: E402

BOX_TOLERANCE_FEET = 0.01


def unfilled(path, boxes=True):
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
            if not boxes:
                raise ValueError("no box wanted")
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
            "predefined": getattr(opening, "PredefinedType", None),
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
    # An opening rvt-rs writes that Revit's export of this file does not, but
    # its export of a sibling file (the same building in another state) does,
    # is counted apart: the two files' records for it can be the same where
    # the exports are not (RE-321 section 4, the maintainer's decision of
    # 2026-11-03, as RE-253 for a held-out wall).
    sibling_keys = {}
    for other in siblings(args[1]):
        for key in unfilled(other, boxes=False):
            sibling_keys.setdefault(key, os.path.basename(other))
    missing = {key: mine for key, mine in ours.items() if key not in revit}
    apart = {key: mine for key, mine in missing.items() if key in sibling_keys}
    extra = collections.Counter(
        f"{mine['cutter']} in {mine['host']}" for key, mine in missing.items() if key not in apart
    )
    print(f"written that Revit's export does not: {sum(extra.values())}")
    for kind, count in sorted(extra.items()):
        print(f"  {kind}: {count}")
    print(f"written that Revit's export does not but its export of a sibling file does, counted apart: {len(apart)}")
    for key, mine in sorted(apart.items()):
        print(f"  {mine['cutter']} in {mine['host']}, Tag {key[0]} in {key[1]}: {sibling_keys[key]}")
    for diff, key, kind in sorted(worst, reverse=True)[:60]:
        print(f"  box differs by {diff:.3f} ft: {kind}, Tag {key[0]} in {key[1]}")
    for key, mine in sorted(missing.items()):
        if key not in apart:
            print(f"  extra: {mine['cutter']} in {mine['host']}, Tag {key[0]} in {key[1]}")
    # The openings Revit's export writes that rvt-rs's does not (#1600): kind, Tags and box.
    only_revit = {key: theirs for key, theirs in revit.items() if key not in ours}
    print(f"written by Revit's export, not by rvt-rs: {len(only_revit)}")
    for key, theirs in sorted(only_revit.items())[:80]:
        box = theirs["box"]
        text = "no box" if box is None else " ".join(f"{v:.3f}" for v in (*box[0], *box[1]))
        print(f"  revit only: {theirs['cutter']} in {theirs['host']}, Tag {key[0]} in {key[1]}, box {text}")
    for key, mine in sorted(missing.items())[:80]:
        box = mine["box"]
        text = "no box" if box is None else " ".join(f"{v:.3f}" for v in (*box[0], *box[1]))
        print(f"  rvt-rs only: {mine['cutter']} in {mine['host']}, Tag {key[0]} in {key[1]}, box {text}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

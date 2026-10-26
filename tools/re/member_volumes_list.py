#!/usr/bin/env python3
"""Where a curtain mullion's NetVolume is its body's volume (#1016, W7).

For every IfcMember Revit's export gives `Qto_MemberBaseQuantities.NetVolume`
and rvt-rs also writes (by Tag), it compares Revit's NetVolume with the
volume of rvt-rs's body (each extruded solid's profile area times its
depth, summed), and with Revit's own body where that is an extrusion
(directly or through a mapped item). It counts the equal ones (a relative
1e-5, or 1e-6 absolute in SI units) by mullion type, by the kind of solid
Revit writes and by whether Revit's Length is rvt-rs's `Span`, and lists up
to 40 that differ.

Usage:

    python3 tools/re/member_volumes_list.py <rvt-rs.ifc> <revit-export.ifc>
"""

import collections
import os
import sys

import ifcopenshell
import ifcopenshell.util.element
import ifcopenshell.util.placement
import ifcopenshell.util.unit
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from qtos_vs_ifc import quantities  # noqa: E402
from wall_qtos_list import profile, solids  # noqa: E402


def bodies(f):
    """Tag -> (body volume or None, the kinds of its items)."""
    scale = ifcopenshell.util.unit.calculate_unit_scale(f)
    out = {}
    for member in f.by_type("IfcMember"):
        if not member.Tag or member.Representation is None:
            continue
        volume = 0.0
        kinds = []
        for rep in member.Representation.Representations:
            if rep.RepresentationIdentifier != "Body":
                continue
            items = list(rep.Items)
            while items:
                item = items.pop()
                if item.is_a("IfcMappedItem"):
                    items.extend(item.MappingSource.MappedRepresentation.Items)
                    continue
                kinds.append(item.is_a())
                found = list(solids(item))
                if not found:
                    volume = None
                for solid in found:
                    area, _ = profile(solid.SweptArea, scale)
                    if area is None or volume is None:
                        volume = None
                    else:
                        volume += area * solid.Depth * scale
        out[f"tag:{member.Tag}"] = (volume if kinds else None, "+".join(sorted(kinds)) or "-")
    return out


def direction(member):
    """vertical, horizontal or slanted: the world direction of the member's
    body extrusion, through its placement and the solid's position."""
    if member.Representation is None:
        return "-"
    for rep in member.Representation.Representations:
        if rep.RepresentationIdentifier != "Body":
            continue
        for item in rep.Items:
            for solid in solids(item):
                m = np.array(ifcopenshell.util.placement.get_local_placement(member.ObjectPlacement))
                if solid.Position is not None:
                    m = m @ np.array(ifcopenshell.util.placement.get_axis2placement(solid.Position))
                d = m[:3, :3] @ np.array(solid.ExtrudedDirection.DirectionRatios, dtype=float)
                z = abs(d[2]) / (np.linalg.norm(d) or 1)
                return "vertical" if z > 0.999 else ("horizontal" if z < 0.001 else "slanted")
    return "-"


def same(a, b):
    return a is not None and b is not None and abs(a - b) <= max(1e-6, 1e-5 * max(abs(a), abs(b)))


def main(argv):
    args = [a for a in argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    ours_f, theirs_f = ifcopenshell.open(args[0]), ifcopenshell.open(args[1])
    mine, theirs = bodies(ours_f), bodies(theirs_f)
    spans = {}
    for member in ours_f.by_type("IfcMember"):
        common = ifcopenshell.util.element.get_psets(member, psets_only=True).get("Pset_MemberCommon", {})
        if member.Tag and common.get("Span") is not None:
            spans[f"tag:{member.Tag}"] = float(common["Span"]) * ifcopenshell.util.unit.calculate_unit_scale(ours_f)
    types = {f"tag:{m.Tag}": getattr(m, "ObjectType", None) or "-" for m in theirs_f.by_type("IfcMember") if m.Tag}
    ours_direction = {f"tag:{m.Tag}": direction(m) for m in ours_f.by_type("IfcMember") if m.Tag}
    revit = collections.defaultdict(dict)
    for (key, qset, name), (_, value) in quantities(args[1])[0].items():
        if qset == "Qto_MemberBaseQuantities" and value is not None:
            revit[key][name] = value
    rows = [k for k in sorted(revit) if "NetVolume" in revit[k] and k in mine]
    print(f"members with NetVolume in Revit's export that rvt-rs writes: {len(rows)}")
    tallies = {
        "type": collections.Counter(),
        "Revit's solid": collections.Counter(),
        "Length is rvt-rs's Span": collections.Counter(),
        "rvt-rs's direction and Revit's solid": collections.Counter(),
    }
    revit_body_equal = 0
    misses = []
    for k in rows:
        v = revit[k]["NetVolume"]
        ok = same(v, mine[k][0])
        tallies["type"][(types.get(k, "-"), ok)] += 1
        tallies["Revit's solid"][(theirs.get(k, (None, "-"))[1], ok)] += 1
        tallies["Length is rvt-rs's Span"][(same(revit[k].get("Length"), spans.get(k)), ok)] += 1
        tallies["rvt-rs's direction and Revit's solid"][(f"{ours_direction.get(k, '-')} {theirs.get(k, (None, '-'))[1]}", ok)] += 1
        revit_body_equal += same(v, theirs.get(k, (None, ""))[0])
        if not ok:
            misses.append(k)
    print(f"Revit's NetVolume = rvt-rs's body volume: {len(rows) - len(misses)} of {len(rows)}")
    print(f"Revit's NetVolume = Revit's own body volume: {revit_body_equal} of {len(rows)}")
    for label, tally in tallies.items():
        print(f"by {label} (value, equal):")
        for (value, ok), n in sorted(tally.items(), key=lambda kv: (str(kv[0][0]), kv[0][1])):
            print(f"    {value} {ok} x{n}")
    print("differing:")
    for k in misses[:40]:
        r = revit[k]
        print(
            f"    {k} {types.get(k, '-')!r}: Revit L {r.get('Length', float('nan')):.6g} V {r['NetVolume']:.6g}"
            f" body {theirs.get(k, (None, '-'))[0]} ({theirs.get(k, (None, '-'))[1]});"
            f" rvt-rs span {spans.get(k)} body {mine[k][0]} ({mine[k][1]})"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

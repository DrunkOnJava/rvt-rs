#!/usr/bin/env python3
"""What a curtain mullion's NetSurfaceArea is (#1016, W7).

For every IfcMember Revit's export gives `Qto_MemberBaseQuantities.NetSurfaceArea`
and rvt-rs also writes (by Tag), it compares Revit's value with surfaces of
Revit's own body where that is extruded solids (through mapped items) and of
rvt-rs's: the sides (each profile's perimeter times its depth), the closed
surface (the sides and both ends) and the sides with one end. It counts the
equal ones (a relative 1e-5, or 1e-6 absolute in SI units) by Revit's solid
kind and by whether Revit's `NetVolume` is rvt-rs's body volume, tallies
(Revit's value less rvt-rs's sides) over rvt-rs's profile area to the
nearest half, and lists up to 30 that match nothing.

Usage:

    python3 tools/re/member_surfaces_list.py <rvt-rs.ifc> <revit-export.ifc>
"""

import collections
import os
import sys

import ifcopenshell
import ifcopenshell.util.unit

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from qto_candidates_list import profile_measures, same  # noqa: E402
from qtos_vs_ifc import quantities  # noqa: E402
from wall_qtos_list import solids  # noqa: E402


def surfaces(f):
    """Tag -> (side, closed, area, volume, kinds) of each IfcMember's body."""
    scale = ifcopenshell.util.unit.calculate_unit_scale(f)
    out = {}
    for member in f.by_type("IfcMember"):
        if not member.Tag or member.Representation is None:
            continue
        side = area = volume = 0.0
        kinds = []
        ok = True
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
                    ok = False
                for solid in found:
                    m = profile_measures(solid.SweptArea, scale)
                    if m is None:
                        ok = False
                        continue
                    depth = solid.Depth * scale
                    side += m[1] * depth
                    area += m[0]
                    volume += m[0] * depth
        kind = "+".join(sorted(kinds)) or "-"
        out[f"tag:{member.Tag}"] = ((side, side + 2 * area, area, volume) if ok and kinds else None, kind)
    return out


def main(argv):
    args = [a for a in argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    mine = surfaces(ifcopenshell.open(args[0]))
    theirs = surfaces(ifcopenshell.open(args[1]))
    revit = collections.defaultdict(dict)
    for (key, qset, name), (_, value) in quantities(args[1])[0].items():
        if qset == "Qto_MemberBaseQuantities" and value is not None:
            revit[key][name] = value
    rows = [k for k in sorted(revit) if "NetSurfaceArea" in revit[k] and k in mine]
    print(f"members with NetSurfaceArea in Revit's export that rvt-rs writes: {len(rows)}")
    tally = collections.Counter()
    ends = collections.Counter()
    unmatched = []
    for k in rows:
        v = revit[k]["NetSurfaceArea"]
        ours, _ = mine[k]
        rv, kind = theirs.get(k, (None, "-"))
        volume_ok = ours is not None and same(revit[k].get("NetVolume"), ours[3])
        labels = []
        if rv is not None:
            labels += [n for n, x in (("Revit closed", rv[1]), ("Revit sides", rv[0])) if same(v, x)]
        if ours is not None:
            side, closed, area, _ = ours
            labels += [n for n, x in (("rvt-rs closed", closed), ("rvt-rs sides", side), ("rvt-rs sides + one end", side + area)) if same(v, x)]
            if area > 0:
                ends[(round(2 * (v - side) / area) / 2, volume_ok)] += 1
        for label in labels or ["none"]:
            tally[(kind, volume_ok, label)] += 1
        if not labels:
            unmatched.append(k)
    print("by Revit's solid, NetVolume = rvt-rs's body volume, and what NetSurfaceArea equals:")
    for (kind, ok, label), n in sorted(tally.items(), key=lambda kv: -kv[1]):
        print(f"  {kind} volume-equal={ok} {label}: {n}")
    print("(Revit's NetSurfaceArea - rvt-rs's sides) / rvt-rs's profile area, to the nearest half, and volume-equal:")
    for (e, ok), n in sorted(ends.items(), key=lambda kv: -kv[1])[:20]:
        print(f"  {e:+.1f} {ok}: {n}")
    print("matching nothing:")
    for k in unmatched[:30]:
        ours, _ = mine[k]
        rv, kind = theirs.get(k, (None, "-"))
        print(f"  {k} Revit {revit[k]['NetSurfaceArea']:.6g} ({kind}, Revit body {rv}); rvt-rs {ours}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

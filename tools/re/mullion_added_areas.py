#!/usr/bin/env python3
"""For the curtain mullions whose NetSurfaceArea one export adds to another's, check that it is what the rule says
(#1046, RE-404): the lateral area of the mullion's body plus a whole number of its flat ends, or, of a mitred one,
its mitre face and, where the end is kept, its flat end.

For models no Revit export scores (Autodesk's samples), where a change that writes a value cannot be read against
Revit's: it reads the two files' kept exports, takes each `IfcMember` of PredefinedType MULLION that holds a
`NetSurfaceArea` in the second and none in the first, measures its mesh along the axis it is a prism along nearest
its `Length` (`mullion_profiles_vs_ifc`'s reading) and prints, by the ends of the body, how many added values are
which sum of its faces, and the first few that are none.

Usage:

    python3 tools/re/mullion_added_areas.py <before.ifc> <after.ifc>

Needs IfcOpenShell and numpy.
"""

import collections
import os
import sys

import ifcopenshell
import ifcopenshell.geom

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mullion_profiles_vs_ifc as mullions  # noqa: E402

LISTED = 8


def main():
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    before, after = ifcopenshell.open(sys.argv[1]), ifcopenshell.open(sys.argv[2])
    scales_before, scales_after = mullions.scales(before), mullions.scales(after)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    held = {str(m.Tag): m for m in before.by_type("IfcMember") if mullions.is_mullion(m)}
    tally, odd, added = collections.Counter(), [], 0
    for member in after.by_type("IfcMember"):
        if not mullions.is_mullion(member):
            continue
        tag = str(member.Tag)
        now = mullions.values(member, scales_after)
        was = mullions.values(held[tag], scales_before) if tag in held else None
        if now["NetSurfaceArea"] is None or (was is not None and was["NetSurfaceArea"] is not None):
            continue
        added += 1
        triangles = mullions.mesh(settings, member)
        axis = mullions.prism_axis(triangles, now["Length"]) if triangles is not None and now["Length"] else None
        if axis is None:
            tally[("no prism axis", ())] += 1
            odd.append((tag, now, None))
            continue
        body = mullions.analyse(triangles, axis)
        sums = mullions.decompose(body, now["NetSurfaceArea"])
        tally[(body["ends"], sums)] += 1
        if not sums:
            odd.append((tag, now, body))
    print(f"mullions with a NetSurfaceArea the first export lacks: {added}")
    for key, count in tally.most_common():
        print(f"  {count:5}  {key}")
    for tag, now, body in odd[:LISTED]:
        print(f"  none of the sums: mullion {tag} {now} {body}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

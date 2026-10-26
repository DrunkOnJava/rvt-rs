#!/usr/bin/env python3
"""Revit's and rvt-rs's base quantities for each curtain mullion, by type
(#1016, W7).

Reads Revit's IFC export and the rvt-rs export next to the probe's output
(`model.ifc` in the probe output's directory; the probe's own output is
ignored). For each IfcMember Revit writes `Qto_MemberBaseQuantities` on, it
takes its quantities in SI units, and rvt-rs's for the same Tag. Per
ObjectType (the mullion type) it prints how many members there are, how
many of each quantity are equal (relative 1e-5, or 1e-6 absolute), and for
the first member of the type Revit's and rvt-rs's values, plus Revit's
CrossSectionArea divided by Length and NetVolume divided by
CrossSectionArea (the extrusion's other two sides, if the area is a side's).

Usage:

    python3 tools/re/member_qtos_list.py <probe.txt> <revit.ifc>
"""

import collections
import os
import sys

import ifcopenshell
import ifcopenshell.util.unit

NAMES = ("Length", "CrossSectionArea", "NetSurfaceArea", "NetVolume")


def quantities(f):
    scale = {
        "LengthValue": ifcopenshell.util.unit.calculate_unit_scale(f, "LENGTHUNIT"),
        "AreaValue": ifcopenshell.util.unit.calculate_unit_scale(f, "AREAUNIT"),
        "VolumeValue": ifcopenshell.util.unit.calculate_unit_scale(f, "VOLUMEUNIT"),
    }
    out = {}
    for rel in f.by_type("IfcRelDefinesByProperties"):
        d = rel.RelatingPropertyDefinition
        if not d.is_a("IfcElementQuantity") or d.Name != "Qto_MemberBaseQuantities":
            continue
        values = {}
        for q in d.Quantities or ():
            for attr, s in scale.items():
                v = getattr(q, attr, None)
                if v is not None:
                    values[q.Name] = float(v) * s
        for obj in rel.RelatedObjects:
            if obj.is_a("IfcMember") and obj.Tag:
                out[obj.Tag] = (getattr(obj, "ObjectType", None) or "-", values)
    return out


def same(a, b):
    return a is not None and b is not None and abs(a - b) <= max(1e-6, 1e-5 * max(abs(a), abs(b)))


def main():
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    ours_path = os.path.join(os.path.dirname(os.path.abspath(args[0])), "model.ifc")
    theirs = quantities(ifcopenshell.open(args[1]))
    ours = quantities(ifcopenshell.open(ours_path)) if os.path.exists(ours_path) else {}
    by_type = collections.defaultdict(list)
    for tag, (kind, values) in theirs.items():
        by_type[kind].append((tag, values, ours.get(tag, ("-", {}))[1]))
    for kind, members in sorted(by_type.items(), key=lambda kv: -len(kv[1])):
        equal = collections.Counter()
        for _, revit, mine in members:
            for name in NAMES:
                equal[name] += same(revit.get(name), mine.get(name))
        tag, revit, mine = members[0]
        length = revit.get("Length")
        area = revit.get("CrossSectionArea")
        volume = revit.get("NetVolume")
        side = area / length if area and length else None
        depth = volume / area if volume and area else None
        print(
            f"{kind!r}: {len(members)} members; equal "
            + ", ".join(f"{n} {equal[n]}" for n in NAMES)
        )
        print(
            f"    e.g. {tag}: Revit "
            + ", ".join(f"{n} {revit.get(n, float('nan')):.6g}" for n in NAMES)
            + f"; area/length {side if side is None else round(side, 6)}, volume/area {depth if depth is None else round(depth, 6)}"
        )
        print("         rvt-rs " + ", ".join(f"{n} {mine.get(n, float('nan')):.6g}" for n in NAMES))
    return 0


if __name__ == "__main__":
    sys.exit(main())

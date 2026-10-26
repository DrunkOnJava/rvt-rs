#!/usr/bin/env python3
"""Which of Revit's base quantities equal a value its export writes beside
them (#1016, W7).

Reads Revit's IFC export (the probe's output is ignored). For each pair
below, on every element carrying both, it counts how many are equal, in SI
units from the file's own unit assignment (relative 1e-5, or 1e-6 absolute),
and prints up to 5 that are not. A quantity that equals a value rvt-rs
already writes as Revit does can be written from it.

Usage:

    python3 tools/re/qto_sources_list.py <probe.txt> <revit.ifc>
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.util.element
import ifcopenshell.util.unit


def main():
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    f = ifcopenshell.open(args[1])
    metres = ifcopenshell.util.unit.calculate_unit_scale(f)

    def qtos(element):
        out = {}
        for rel in getattr(element, "IsDefinedBy", None) or ():
            if not rel.is_a("IfcRelDefinesByProperties"):
                continue
            d = rel.RelatingPropertyDefinition
            if d.is_a("IfcElementQuantity"):
                for q in d.Quantities or ():
                    for attr, power in (("LengthValue", 1), ("AreaValue", 2), ("VolumeValue", 3)):
                        v = getattr(q, attr, None)
                        if v is not None:
                            out[f"{d.Name}.{q.Name}"] = float(v) * metres**power
        return out

    def pset(element, name):
        set_name, _, prop = name.partition(".")
        v = ifcopenshell.util.element.get_psets(element, psets_only=True).get(set_name, {}).get(prop)
        return float(v) * metres if isinstance(v, (int, float)) and not isinstance(v, bool) else None

    def attribute(element, name):
        v = getattr(element, name, None)
        return float(v) * metres if v is not None else None

    pairs = [
        ("IfcMember", "Qto_MemberBaseQuantities.Length", lambda e: pset(e, "Pset_MemberCommon.Span")),
        ("IfcDoor", "Qto_DoorBaseQuantities.Width", lambda e: attribute(e, "OverallWidth")),
        ("IfcDoor", "Qto_DoorBaseQuantities.Height", lambda e: attribute(e, "OverallHeight")),
        ("IfcWindow", "Qto_WindowBaseQuantities.Width", lambda e: attribute(e, "OverallWidth")),
        ("IfcWindow", "Qto_WindowBaseQuantities.Height", lambda e: attribute(e, "OverallHeight")),
        ("IfcRailing", "Qto_RailingBaseQuantities.Length", lambda e: None),
        ("IfcPipeSegment", "Qto_PipeSegmentBaseQuantities.Length", lambda e: None),
    ]
    areas = [
        ("IfcDoor", "Qto_DoorBaseQuantities.Area", ("Qto_DoorBaseQuantities.Width", "Qto_DoorBaseQuantities.Height")),
        ("IfcWindow", "Qto_WindowBaseQuantities.Area", ("Qto_WindowBaseQuantities.Width", "Qto_WindowBaseQuantities.Height")),
        ("IfcDoor", "Qto_DoorBaseQuantities.Area", ("OverallWidth", "OverallHeight")),
        ("IfcWindow", "Qto_WindowBaseQuantities.Area", ("OverallWidth", "OverallHeight")),
    ]

    def same(a, b):
        return abs(a - b) <= max(1e-6, 1e-5 * max(abs(a), abs(b)))

    tally = collections.defaultdict(collections.Counter)
    misses = collections.defaultdict(list)
    for cls, qname, other in pairs:
        for e in f.by_type(cls):
            q = qtos(e).get(qname)
            o = other(e)
            if q is None or o is None:
                continue
            key = (cls, qname, "beside")
            tally[key]["both"] += 1
            if same(q, o):
                tally[key]["equal"] += 1
            elif len(misses[key]) < 5:
                misses[key].append(f"{e.Tag} {q:.6g} vs {o:.6g}")
    for cls, qname, (a, b) in areas:
        for e in f.by_type(cls):
            values = qtos(e)
            q = values.get(qname)
            x = values.get(a, attribute(e, a))
            y = values.get(b, attribute(e, b))
            if q is None or x is None or y is None:
                continue
            key = (cls, qname, f"{a} x {b}")
            tally[key]["both"] += 1
            if same(q, x * y):
                tally[key]["equal"] += 1
            elif len(misses[key]) < 5:
                misses[key].append(f"{e.Tag} {q:.6g} vs {x * y:.6g}")
    for key in sorted(tally):
        print(f"  {key[0]:<10} {key[1]:<40} {key[2]:<60} {tally[key]['equal']} of {tally[key]['both']}")
        for line in misses[key]:
            print(f"      {line}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

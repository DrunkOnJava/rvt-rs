#!/usr/bin/env python3
"""Score the PredefinedType rvt-rs writes on stairs and their parts against
Revit's own IFC export (#323).

Pairs each IfcStair, IfcStairFlight, IfcSlab and IfcMember of the two files
by Tag and counts, per entity, how often the PredefinedType is the same,
differs, or is unset in rvt-rs's, with Revit's values tallied, and names a
few that differ.

Needs IfcOpenShell.

Usage:

    python3 tools/re/predefined_types_vs_ifc.py <rvt-rs.ifc> <revit.ifc>
"""

import collections
import sys

import ifcopenshell

CLASSES = ("IfcStair", "IfcStairFlight", "IfcSlab", "IfcMember")


def predefined(path):
    model = ifcopenshell.open(path)
    out = {}
    for cls in CLASSES:
        for element in model.by_type(cls):
            if element.is_a() == cls and element.Tag:
                out[(cls, str(element.Tag))] = getattr(element, "PredefinedType", None) or "-"
    return out


def type_values(path):
    model = ifcopenshell.open(path)
    return collections.Counter(
        getattr(t, "PredefinedType", None) or "-" for t in model.by_type("IfcStairFlightType")
    )


def main():
    ours, revit = predefined(sys.argv[1]), predefined(sys.argv[2])
    print(
        f"IfcStairFlightType PredefinedType: rvt-rs {dict(sorted(type_values(sys.argv[1]).items()))}, "
        f"Revit {dict(sorted(type_values(sys.argv[2]).items()))}"
    )
    for cls in CLASSES:
        keys = [k for k in revit if k[0] == cls and k in ours]
        if not keys:
            continue
        tally = collections.Counter()
        values = collections.Counter(revit[k] for k in keys)
        misses = []
        for key in keys:
            mine, theirs = ours[key], revit[key]
            if mine == theirs:
                tally["same"] += 1
            elif mine in ("-", "NOTDEFINED"):
                tally["unset in rvt-rs"] += 1
                misses.append((key[1], mine, theirs))
            else:
                tally["differs"] += 1
                misses.append((key[1], mine, theirs))
        print(
            f"{cls}: {len(keys)} in both; same {tally['same']}, differs {tally['differs']}, "
            f"unset in rvt-rs {tally['unset in rvt-rs']}; Revit's: {dict(sorted(values.items()))}"
        )
        for tag, mine, theirs in misses[:6]:
            print(f"  {tag}: rvt-rs {mine}, Revit {theirs}")


if __name__ == "__main__":
    main()

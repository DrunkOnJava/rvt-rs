#!/usr/bin/env python3
"""Score the GlobalIds rvt-rs writes against Revit's own IFC export.

Research tool for `reports/element-framing/RE-108-global-ids-2023.md`
(RE-48 on Revit 2023). Elements are paired by Tag (the Revit ElementId),
leaving out opening elements, which carry their door's or window's Tag but
a GlobalId of Revit's own making. Storeys are paired by name, and spaces by
GlobalId.

Usage:

    python3 tools/re/global_ids_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>

Needs IfcOpenShell (tested with 0.8.5).
"""

import collections
import sys

import ifcopenshell


def by_tag(f):
    out = {}
    for element in f.by_type("IfcElement"):
        if element.Tag and not element.is_a("IfcOpeningElement"):
            out.setdefault(element.Tag, set()).add(element.GlobalId)
    return out


def main(argv):
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    ours, revit = ifcopenshell.open(argv[1]), ifcopenshell.open(argv[2])
    counts = collections.Counter()
    theirs = by_tag(revit)
    for tag, global_ids in by_tag(ours).items():
        if tag not in theirs:
            counts["not in Revit's export"] += 1
        else:
            counts["GlobalId = Revit's" if global_ids & theirs[tag] else "differs"] += 1
    print("elements:", dict(counts))
    revit_storeys = {s.Name: s.GlobalId for s in revit.by_type("IfcBuildingStorey")}
    our_storeys = {s.Name: s.GlobalId for s in ours.by_type("IfcBuildingStorey")}
    equal = sum(1 for name, gid in our_storeys.items() if revit_storeys.get(name) == gid)
    print(f"storeys: {equal} of {len(our_storeys)} equal Revit's")
    our_spaces = {s.GlobalId for s in ours.by_type("IfcSpace")}
    revit_spaces = {s.GlobalId for s in revit.by_type("IfcSpace")}
    print(f"spaces: {len(our_spaces & revit_spaces)} of {len(our_spaces)} equal one of Revit's")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

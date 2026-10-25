#!/usr/bin/env python3
"""Score the GlobalIds rvt-rs writes against Revit's own IFC export.

Research tool for `reports/element-framing/RE-108-global-ids-2023.md`
(RE-48 on Revit 2023). Elements are paired by Tag (the Revit ElementId),
leaving out opening elements, which carry their door's or window's Tag but
a GlobalId of Revit's own making. Storeys are paired by name, and spaces by
GlobalId.

Usage:

    python3 tools/re/global_ids_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
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
    # A model renamed after its export (BIMcollab's 2015 example) pairs no
    # storey by name; a storey whose GlobalId is one of Revit's still counts.
    revit_storey_ids = set(revit_storeys.values())
    print(f"storeys whose GlobalId is one of Revit's: {sum(1 for gid in our_storeys.values() if gid in revit_storey_ids)} of {len(our_storeys)}")
    for entity in ("IfcProject", "IfcSite", "IfcBuilding"):
        theirs_ids = {e.GlobalId for e in revit.by_type(entity)}
        ours_ids = [e.GlobalId for e in ours.by_type(entity)]
        print(f"{entity}: {sum(1 for gid in ours_ids if gid in theirs_ids)} of {len(ours_ids)} equal Revit's")
    our_spaces = {s.GlobalId for s in ours.by_type("IfcSpace")}
    revit_spaces = {s.GlobalId for s in revit.by_type("IfcSpace")}
    print(f"spaces: {len(our_spaces & revit_spaces)} of {len(our_spaces)} equal one of Revit's")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

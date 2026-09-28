#!/usr/bin/env python3
"""Compare the host walls of doors and windows in an rvt-rs IFC export with
Revit's own IFC4 export.

Research tool for `reports/element-framing/RE-85-opening-host-candidates.md`
(#439). Keyed by ElementId (`Tag`). For every door and window rvt-rs
exports, it says whether its opening's host (`IfcRelFillsElement` then
`IfcRelVoidsElement`) is the wall Revit's filled opening voids, another wall
Revit also cuts for that element, or another wall; and for unhosted ones
what Revit does. It also counts rvt-rs's openings with no filler (RE-84) and
how many void the wall Revit's opening with that Tag voids.

Usage:

    python3 tools/re/opening_hosts_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>

Needs IfcOpenShell (tested with 0.8.5).
"""

import collections
import sys

import ifcopenshell


def hosts(model):
    """Filler Tag -> the wall its opening voids; Tag -> every wall voided."""
    fills = {}
    voids = collections.defaultdict(set)
    for rel in model.by_type("IfcRelFillsElement"):
        for void in rel.RelatingOpeningElement.VoidsElements:
            fills[rel.RelatedBuildingElement.Tag] = void.RelatingBuildingElement.Tag
    for opening in model.by_type("IfcOpeningElement"):
        for void in opening.VoidsElements:
            voids[opening.Tag].add(void.RelatingBuildingElement.Tag)
    return fills, voids


def main(argv):
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    ours, revit = ifcopenshell.open(argv[1]), ifcopenshell.open(argv[2])
    our_fills, _ = hosts(ours)
    revit_fills, revit_voids = hosts(revit)
    exported = {
        e.Tag for e in revit.by_type("IfcElement") if not e.is_a("IfcOpeningElement")
    }
    outcomes = collections.Counter()
    listed = []
    for element in ours.by_type("IfcDoor") + ours.by_type("IfcWindow"):
        tag = element.Tag
        host = our_fills.get(tag)
        if host is None:
            if tag in revit_fills:
                outcomes["no host; Revit fills an opening"] += 1
            elif tag in revit_voids:
                outcomes["no host; Revit writes an opening only"] += 1
            else:
                outcomes["no host; Revit writes no opening"] += 1
        elif revit_fills.get(tag) == host:
            outcomes["the wall Revit fills"] += 1
        elif host in revit_voids.get(tag, ()):
            outcomes["another wall Revit cuts"] += 1
            listed.append(tag)
        else:
            outcomes["a wall Revit does not cut"] += 1
            listed.append(tag)
    bare = [o for o in ours.by_type("IfcOpeningElement") if not o.HasFillings]
    outcomes["openings with no filler"] = len(bare)
    outcomes["openings with no filler voiding Revit's wall"] = sum(
        1
        for opening in bare
        for void in opening.VoidsElements
        if void.RelatingBuildingElement.Tag in revit_voids.get(opening.Tag, ())
    )
    outcomes["doors and windows not in Revit's export"] = sum(
        1
        for element in ours.by_type("IfcDoor") + ours.by_type("IfcWindow")
        if element.Tag not in exported
    )
    for outcome, count in outcomes.items():
        print(f"{outcome}: {count}")
    if listed:
        print("not the wall Revit fills:", " ".join(sorted(listed)))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

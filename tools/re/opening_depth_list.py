#!/usr/bin/env python3
"""Which of rvt-rs's door and window openings Revit's export gives a Depth
(#1016, RE-244).

For each opening rvt-rs's IFC writes `Qto_OpeningElementBaseQuantities.Depth`
on, found by the door or window filling it (`IfcRelFillsElement`), it finds
Revit's opening for the same filler, and counts, by the filler's class and
whether Revit's opening voids an element of the same class and Tag as
rvt-rs's: how many Revit gives a Depth, how many equal rvt-rs's (a relative
1e-5, or 1e-6 absolute in SI units), and lists up to 12 of the rest.

Usage:

    python3 tools/re/opening_depth_list.py <rvt-rs.ifc> <revit-export.ifc>
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.util.unit


def openings(f):
    """filler Tag -> (filler class, host class, host Tag, Depth in metres or None)."""
    lengths = ifcopenshell.util.unit.calculate_unit_scale(f, "LENGTHUNIT")
    host_of = {rel.RelatedOpeningElement.id(): rel.RelatingBuildingElement for rel in f.by_type("IfcRelVoidsElement")}
    out = {}
    for rel in f.by_type("IfcRelFillsElement"):
        opening, fill = rel.RelatingOpeningElement, rel.RelatedBuildingElement
        if not fill.Tag:
            continue
        depth = None
        for r in getattr(opening, "IsDefinedBy", None) or ():
            d = r.RelatingPropertyDefinition if r.is_a("IfcRelDefinesByProperties") else None
            if d is not None and d.is_a("IfcElementQuantity") and d.Name == "Qto_OpeningElementBaseQuantities":
                for q in d.Quantities or ():
                    if q.Name == "Depth" and q.is_a("IfcQuantityLength"):
                        depth = float(q.LengthValue) * lengths
        host = host_of.get(opening.id())
        out[fill.Tag] = (fill.is_a(), host.is_a() if host else "-", getattr(host, "Tag", None), depth)
    return out


def same(a, b):
    return a is not None and b is not None and abs(a - b) <= max(1e-6, 1e-5 * max(abs(a), abs(b)))


def main(argv):
    args = [a for a in argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    ours_f, theirs_f = ifcopenshell.open(args[0]), ifcopenshell.open(args[1])
    mine, theirs = openings(ours_f), openings(theirs_f)
    tally = collections.Counter()
    rest = []
    for tag, (cls, host_cls, host_tag, depth) in sorted(mine.items()):
        if depth is None:
            continue
        revit = theirs.get(tag)
        if revit is None:
            tally[(cls, "Revit fills no opening with it", "-")] += 1
            continue
        same_host = (revit[1], revit[2]) == (host_cls, host_tag)
        if revit[3] is None:
            state = "Revit writes no Depth"
        else:
            state = "equal" if same(depth, revit[3]) else "different"
        tally[(cls, f"same host {same_host}", state)] += 1
        if state != "equal" and len(rest) < 12:
            rest.append(f"{tag} {cls}: rvt-rs {depth:.6g} in {host_cls} {host_tag}; Revit {revit[3]} in {revit[1]} {revit[2]}")
    print(f"rvt-rs openings with a Depth: {sum(tally.values())}")
    for key, n in sorted(tally.items()):
        print(f"    {key[0]} {key[1]} {key[2]} x{n}")
    for line in rest:
        print(f"    {line}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

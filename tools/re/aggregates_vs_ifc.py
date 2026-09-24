#!/usr/bin/env python3
"""Compare the whole each part of an rvt-rs IFC is aggregated under with the
whole Revit's own IFC export of the same file aggregates it under.

Research tool for `reports/element-framing/RE-72-curtain-grids.md` (and
RE-46, RE-62). A part's whole is the `RelatingObject` of the
`IfcRelAggregates` that lists it; elements are matched by `Tag`, the Revit
ElementId. Only curtain-wall parts are counted: `IfcMember`, `IfcPlate` and
`IfcCurtainWall` (a nested curtain wall is itself a part).

Usage:

    python3 tools/re/aggregates_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc> [-v]

`-v` lists the parts whose whole differs. Needs IfcOpenShell (tested with
0.8.5).
"""

import collections
import sys

import ifcopenshell

KINDS = ("IfcMember", "IfcPlate", "IfcCurtainWall")


def tag(entity):
    try:
        return entity.Tag
    except AttributeError:
        return None


def wholes(model):
    out = {}
    for rel in model.by_type("IfcRelAggregates"):
        whole = rel.RelatingObject
        for part in rel.RelatedObjects:
            if tag(part) and not part.is_a("IfcOpeningElement"):
                out[tag(part)] = (tag(whole), whole.is_a())
    return out


def main():
    args = [a for a in sys.argv[1:] if a != "-v"]
    verbose = "-v" in sys.argv[1:]
    if len(args) != 2:
        raise SystemExit(__doc__)
    ours = ifcopenshell.open(args[0])
    theirs = ifcopenshell.open(args[1])
    ours_wholes, theirs_wholes = wholes(ours), wholes(theirs)
    in_theirs = {tag(e) for e in theirs.by_type("IfcElement") if tag(e)}
    counts = collections.Counter()
    listed = []
    for element in ours.by_type("IfcElement"):
        kind = element.is_a()
        key = tag(element)
        if kind not in KINDS or not key:
            continue
        a, b = ours_wholes.get(key), theirs_wholes.get(key)
        if key not in in_theirs:
            verdict = "not in Revit's export"
        elif a is None and b is None:
            verdict = "neither aggregates"
        elif a is None:
            verdict = "only Revit aggregates"
        elif b is None:
            verdict = "only rvt-rs aggregates"
        elif a[0] == b[0]:
            verdict = "same whole"
        else:
            verdict = "different whole"
        counts[(kind, verdict)] += 1
        if verdict in ("only Revit aggregates", "different whole"):
            listed.append((key, kind, a, b))
    for (kind, verdict), count in sorted(counts.items()):
        print(f"  {count:>5}  {kind}: {verdict}")
    if verbose:
        for key, kind, a, b in listed:
            print(f"    {key} {kind}: rvt-rs {a}, Revit {b}")


if __name__ == "__main__":
    main()

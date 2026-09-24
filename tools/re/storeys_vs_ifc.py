#!/usr/bin/env python3
"""Compare the storey each element of an rvt-rs IFC is in with the storey
Revit's own IFC export of the same file puts it in.

Research tool for `reports/element-framing/RE-59-base-constraint-levels.md`.

An element's storey is the `IfcBuildingStorey` an
`IfcRelContainedInSpatialStructure` puts it in. A part aggregated under a
whole (a curtain wall's panels and mullions, a stair's flights) takes its
whole's storey. Elements are matched by `Tag`, the Revit ElementId.
Containment in anything other than a storey (the building, a space) counts
as no storey for rvt-rs and as that container for Revit.

With only the rvt-rs file, it counts the elements left off a storey.

Usage:

    python3 tools/re/storeys_vs_ifc.py [-v] <rvt-rs.ifc> [<revit-export.ifc>]

`-v` prints the elements whose storeys differ. Needs IfcOpenShell (tested
with 0.8.5).
"""

import collections
import sys

import ifcopenshell


def tag(entity):
    try:
        return entity.Tag
    except AttributeError:
        return None


def storeys(model):
    """Each element's storey name by Tag, or `<type>` for another container."""
    by_id = {}
    for rel in model.by_type("IfcRelContainedInSpatialStructure"):
        where = rel.RelatingStructure
        name = where.Name if where.is_a("IfcBuildingStorey") else f"<{where.is_a()}>"
        for element in rel.RelatedElements:
            by_id[element.id()] = name
    for _ in range(3):
        for rel in model.by_type("IfcRelAggregates"):
            whole = rel.RelatingObject
            if whole.id() in by_id:
                for part in rel.RelatedObjects:
                    by_id.setdefault(part.id(), by_id[whole.id()])
    out = {}
    for entity_id, name in by_id.items():
        key = tag(model.by_id(entity_id))
        if key:
            out[key] = name
    return out


def on_storey(name):
    return name is not None and not name.startswith("<")


def main():
    args = [a for a in sys.argv[1:] if a != "-v"]
    verbose = "-v" in sys.argv[1:]
    if not 1 <= len(args) <= 2:
        raise SystemExit(__doc__)
    ours = ifcopenshell.open(args[0])
    our_storeys = storeys(ours)
    elements = [e for e in ours.by_type("IfcElement") if not e.is_a("IfcOpeningElement")]
    missing = collections.Counter(e.is_a() for e in elements if not on_storey(our_storeys.get(tag(e))))
    print(f"{len(elements)} elements, {sum(missing.values())} not on a storey")
    for kind, count in missing.most_common():
        print(f"  {count:>5}  {kind}")
    if len(args) == 1:
        return
    theirs = storeys(ifcopenshell.open(args[1]))
    verdicts = collections.Counter()
    differ = collections.Counter()
    for element in elements:
        key = tag(element)
        if key not in theirs:
            continue
        mine, revit = our_storeys.get(key), theirs[key]
        if mine == revit:
            verdicts["same storey as Revit's"] += 1
        elif not on_storey(mine):
            verdicts["on no storey; Revit's is " + ("a storey" if on_storey(revit) else "another container")] += 1
        elif not on_storey(revit):
            verdicts[f"on a storey; Revit contains it in {revit}"] += 1
        else:
            verdicts["a different storey from Revit's"] += 1
            differ[element.is_a()] += 1
            if verbose:
                print(f"    {key} {element.is_a()}: {mine} vs Revit's {revit}")
    print("against Revit's export:")
    for what, count in verdicts.most_common():
        print(f"  {count:>5}  {what}")
    if differ:
        print("  different, by type: " + ", ".join(f"{k} {v}" for k, v in differ.most_common()))


if __name__ == "__main__":
    main()

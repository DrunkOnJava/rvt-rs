#!/usr/bin/env python3
"""List the property sets of Revit's own IFC export, set by set.

Research tool for B40 (#35, #85): which of Revit's property sets carry
project and shared parameters, and how many values each writes. For every
`IfcPropertySet` related to an object, it counts the objects and values of
each (set, property), with the value types written. `Pset_` sets are listed
apart from the rest, which hold Revit's own parameter groups.

Usage (as a Measure probe scorer, the probe output is ignored):

    python3 tools/re/reference_property_sets.py <probe.txt> <revit-export.ifc> [--list]

Needs IfcOpenShell (tested with 0.8.5).
"""

import collections
import sys

import ifcopenshell


def main(argv):
    args = [a for a in argv[1:] if not a.startswith("--")]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    f = ifcopenshell.open(args[1])
    values = collections.Counter()
    types = collections.defaultdict(collections.Counter)
    objects = collections.defaultdict(set)
    for rel in f.by_type("IfcRelDefinesByProperties"):
        pset = rel.RelatingPropertyDefinition
        if not pset.is_a("IfcPropertySet"):
            continue
        for prop in pset.HasProperties or ():
            key = (pset.Name or "", prop.Name or "")
            kind = prop.is_a()
            if prop.is_a("IfcPropertySingleValue") and prop.NominalValue is not None:
                kind = prop.NominalValue.is_a()
            for obj in rel.RelatedObjects:
                values[key] += 1
                types[key][kind] += 1
                objects[key[0]].add(obj.id())
    standard = sorted(k for k in values if k[0].startswith("Pset_"))
    other = sorted(k for k in values if not k[0].startswith("Pset_"))
    print(
        f"sets: {len({k[0] for k in values})}; Pset_ values: {sum(values[k] for k in standard)}; "
        f"other values: {sum(values[k] for k in other)}"
    )
    for title, keys in (("other sets", other), ("Pset_ sets", standard)):
        print(title)
        last = None
        for key in keys:
            if key[0] != last:
                print(f"  {key[0]}: {len(objects[key[0]])} objects")
                last = key[0]
            kinds = ", ".join(f"{k} {n}" for k, n in types[key].most_common())
            print(f"    {key[1]}: {values[key]} ({kinds})")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

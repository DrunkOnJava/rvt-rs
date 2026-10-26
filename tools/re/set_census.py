#!/usr/bin/env python3
"""Every property set and quantity set Revit's export writes, by name
(RE-239, #35, W7).

Reads Revit's IFC export (the probe's output is ignored). For each set name
it prints how many objects carry it, how many values it holds on them, the
entities it is on and whether buildingSMART names it (`Pset_`, `Qto_`) or
the export does (any other name: a user-defined property set file's sets,
Revit property sets named for parameter groups, the exporter's own).

Usage:

    python3 tools/re/set_census.py <probe.txt> <revit.ifc>
"""

import collections
import sys

import ifcopenshell


def main():
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    f = ifcopenshell.open(args[1])
    objects = collections.Counter()
    values = collections.Counter()
    entities = collections.defaultdict(collections.Counter)
    for rel in f.by_type("IfcRelDefinesByProperties"):
        definition = rel.RelatingPropertyDefinition
        if definition.is_a("IfcPropertySet"):
            held = len(definition.HasProperties or ())
        elif definition.is_a("IfcElementQuantity"):
            held = len(definition.Quantities or ())
        else:
            continue
        name = definition.Name or "-"
        for obj in rel.RelatedObjects:
            objects[name] += 1
            values[name] += held
            entities[name][obj.is_a()] += 1
    for kind in ("Pset_", "Qto_", "other"):
        names = [
            n for n in values
            if (n.startswith(kind) if kind != "other" else not n.startswith(("Pset_", "Qto_")))
        ]
        print(f"{kind}: {len(names)} sets, {sum(objects[n] for n in names)} objects, {sum(values[n] for n in names)} values")
        for name in sorted(names, key=lambda n: (-values[n], n)):
            on = ", ".join(f"{c} x{k}" for c, k in entities[name].most_common(4))
            print(f"  {name}: {objects[name]} objects, {values[name]} values; on {on}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

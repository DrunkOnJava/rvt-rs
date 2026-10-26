#!/usr/bin/env python3
"""Every element Revit's export writes for a ramp (RE-240, #35).

Reads Revit's IFC export (the probe's output is ignored). For each IfcRamp
and IfcRampFlight, and every other element sharing one's Tag, it prints the
Tag, entity, Name, ObjectType, the type object an IfcRelDefinesByType
relates it to (or `-`) and the element aggregating it (or `-`).

Usage:

    python3 tools/re/ramp_elements_list.py <probe.txt> <revit.ifc>
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
    typed = {}
    for rel in f.by_type("IfcRelDefinesByType"):
        for obj in rel.RelatedObjects:
            typed[obj.id()] = rel.RelatingType
    whole = {}
    for rel in f.by_type("IfcRelAggregates"):
        for part in rel.RelatedObjects:
            whole[part.id()] = rel.RelatingObject
    tags = {e.Tag for cls in ("IfcRamp", "IfcRampFlight") for e in f.by_type(cls) if e.Tag}
    by_tag = collections.defaultdict(list)
    for e in f.by_type("IfcElement"):
        if e.Tag in tags:
            by_tag[e.Tag].append(e)
    for tag in sorted(by_tag):
        for e in by_tag[tag]:
            kind = typed.get(e.id())
            parent = whole.get(e.id())
            print(
                f"  {tag} {e.is_a()} {e.Name!r} {getattr(e, 'ObjectType', None)!r} "
                f"type={'-' if kind is None else f'{kind.is_a()}:{kind.Name!r}:{kind.Tag}'} "
                f"whole={'-' if parent is None else f'{parent.is_a()}:{parent.Name!r}'}"
            )
    return 0


if __name__ == "__main__":
    sys.exit(main())

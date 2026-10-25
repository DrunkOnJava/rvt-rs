#!/usr/bin/env python3
"""List Revit's Name and ObjectType for each stair element
`probe_323_stair_names_2025` prints (#323).

For each `elem <id> <class>` line of the probe output, prints the Revit
entities with that Tag: their class, PredefinedType, Name and ObjectType,
and the type object they are typed by, so the native records the probe
prints can be read against the names Revit's export gives.

Usage:

    python3 tools/re/stair_names_list.py <probe output> <revit.ifc> [--list]
"""

import sys

import ifcopenshell


def main():
    elems = [
        line.split()[1:3]
        for line in open(sys.argv[1], encoding="utf-8", errors="replace")
        if line.startswith("elem ")
    ]
    model = ifcopenshell.open(sys.argv[2])
    by_tag = {}
    for element in model.by_type("IfcElement"):
        if getattr(element, "Tag", None):
            by_tag.setdefault(str(element.Tag), []).append(element)
    typed = {}
    for rel in model.by_type("IfcRelDefinesByType"):
        for element in rel.RelatedObjects:
            typed[element.id()] = rel.RelatingType
    for tag, cls in elems:
        found = by_tag.get(tag, [])
        if not found:
            print(f"elem {tag} {cls}: not in Revit's export")
        for element in found:
            kind = typed.get(element.id())
            kind = f"{kind.is_a()} {kind.Name!r}" if kind else "-"
            print(
                f"elem {tag} {cls}: {element.is_a()} {getattr(element, 'PredefinedType', None)}"
                f" Name {element.Name!r} ObjectType {element.ObjectType!r} type {kind}"
            )
    return 0


if __name__ == "__main__":
    sys.exit(main())

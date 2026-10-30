#!/usr/bin/env python3
"""Which record names the type of an element rvt-rs writes untyped (RE-240,
#35, W7).

Reads `probes/examples/probe_re240_untyped_types.rs`'s output and Revit's IFC
export. For each element the probe lists, Revit's element of the same Tag
gives its ObjectType and type object (`IfcRelDefinesByType`); the scorer
says which of the ids the element's native record names is that type
object's Tag, under which path, and whether that id's `SymbolInfo.m_name`
is the type's name. A summary per IFC class counts how many elements each
path gives the right type and name.

Usage:

    python3 tools/re/untyped_types_vs_ifc.py <probe.txt> <revit.ifc>
"""

import collections
import re
import sys

import ifcopenshell


def main():
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    elements = {}
    refs = {}
    for line in open(args[0], encoding="utf-8", errors="replace"):
        parts = line.split()
        if len(parts) >= 5 and parts[0] == "el":
            named = [p.rsplit("@", 1) for p in line.split(" refs=", 1)[1].split()] if " refs=" in line else []
            elements[parts[1]] = (parts[2], parts[3], parts[4].removeprefix("class="), named)
        elif len(parts) >= 2 and parts[0] == "ref":
            fields = dict(re.findall(r"(\w+)=(\S+)", line))
            refs[parts[1]] = fields
    f = ifcopenshell.open(args[1])
    typed = {}
    for rel in f.by_type("IfcRelDefinesByType"):
        for obj in rel.RelatedObjects:
            typed[obj.id()] = rel.RelatingType
    revit = {}
    for element in f.by_type("IfcElement"):
        if element.Tag in elements:
            revit.setdefault(element.Tag, element)
    summary = collections.defaultdict(collections.Counter)
    for tag, (cls, name, native, named) in sorted(elements.items()):
        element = revit.get(tag)
        if element is None:
            summary[cls]["not in Revit's export"] += 1
            continue
        kind = typed.get(element.id())
        type_tag = getattr(kind, "Tag", None) if kind else None
        type_name = kind.Name if kind else None
        hits = [(r, p) for r, p in ((r, p) for r, p in named) if r == type_tag] if type_tag else []
        named_right = [
            (r, p) for r, p in named if refs.get(r, {}).get("name", "-").replace("_", " ") == (type_name or "\0")
        ]
        summary[cls]["elements"] += 1
        summary[cls]["typed by Revit"] += bool(kind)
        summary[cls]["type's id named"] += bool(hits)
        summary[cls]["a named id's SymbolInfo name is the type's"] += bool(named_right)
        for _, p in hits:
            summary[cls][f"type at {re.sub(r'[[][0-9]+[]]', '[]', p)}"] += 1
        print(
            f"  {tag} {cls} {element.is_a()} native={native} Revit {element.Name!r} {element.ObjectType!r} "
            f"type={type_tag}:{type_name!r} hits={[p for _, p in hits]} "
            f"named={[(r, refs.get(r, {}).get('class'), refs.get(r, {}).get('name')) for r, _ in named_right]}"
        )
    for cls in sorted(summary):
        print(f"{cls}: " + ", ".join(f"{k} {v}" for k, v in sorted(summary[cls].items())))
    return 0


if __name__ == "__main__":
    sys.exit(main())

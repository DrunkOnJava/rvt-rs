#!/usr/bin/env python3
"""Score rvt-rs's IFC type objects against Revit's own export (RE-110).

For every element Revit's export types through IfcRelDefinesByType,
keyed by its Tag (the ElementId), compare the type rvt-rs gives the
element with the same Tag: its entity, its Name (`Family:Type`), its Tag
(the type's ElementId) and its GlobalId.

Usage:
    python3 tools/re/type_objects_vs_ifc.py OURS.ifc REVIT.ifc
"""

import collections
import sys

import ifcopenshell

VERBOSE = "-v" in sys.argv
if VERBOSE:
    sys.argv.remove("-v")


def typed(path):
    f = ifcopenshell.open(path)
    out = {}
    for rel in f.by_type("IfcRelDefinesByType"):
        t = rel.RelatingType
        for element in rel.RelatedObjects:
            tag = getattr(element, "Tag", None)
            if tag:
                out.setdefault(tag, (element.is_a(), t.is_a(), t.Name, t.Tag, t.GlobalId))
    return f, out


def main():
    ours_file, ours = typed(sys.argv[1])
    _, revit = typed(sys.argv[2])
    ours_tags = {e.Tag for e in ours_file.by_type("IfcElement") if getattr(e, "Tag", None)}
    fields = ["entity", "name", "tag", "global_id"]
    agree = collections.Counter()
    exported = both = 0
    by_class = collections.Counter()
    wrong = collections.Counter()
    shown = collections.Counter()
    for tag, (occurrence, *expected) in revit.items():
        if tag not in ours_tags:
            continue
        exported += 1
        got = ours.get(tag)
        if got is None:
            by_class[occurrence] += 1
            continue
        both += 1
        for field, e, g in zip(fields, expected, got[1:]):
            if e == g:
                agree[field] += 1
            else:
                wrong[(field, occurrence)] += 1
                if VERBOSE and shown[(field, occurrence)] < 3:
                    shown[(field, occurrence)] += 1
                    print(f"  {field} {occurrence} {tag}: Revit {e!r} ours {g!r}")
    extra = sum(1 for tag in ours if tag not in revit)
    print(f"Revit types {len(revit)} elements; {exported} of them are in ours")
    print(f"typed in ours: {both} of {exported}")
    for field in fields:
        print(f"  {field} agrees: {agree[field]} of {both}")
    print(f"typed in ours but not in Revit's export: {extra}")
    print(f"untyped in ours, by class: {dict(by_class)}")
    if wrong:
        print(f"disagreements: {dict(wrong)}")
    types = {t.Tag for t in ours_file.by_type("IfcTypeObject")}
    print(f"type objects in ours: {len(types)}")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Compare the PredefinedType of every IfcFurniture and IfcFurnitureType of two exports of one file, by GlobalId
(#1681, RE-419).

For models no Revit export scores (Autodesk's samples), where a change to the furniture's PredefinedType cannot be
read against Revit's: it says how many elements and types go from which value to which (`-` is none), whether any
element or type is added or removed, whether a name or a tag differs, and how the count of each entity class
changes. `tools/re/ifc_element_diff.py` reads property and quantity sets and does not see this attribute.

Usage:

    python3 tools/re/furniture_predefined_types_diff.py <before.ifc> <after.ifc>

Needs IfcOpenShell.
"""

import collections
import sys

import ifcopenshell

CLASSES = ("IfcFurniture", "IfcFurnitureType")


def load(path):
    model = ifcopenshell.open(path)
    held = {}
    for cls in CLASSES:
        for element in model.by_type(cls):
            if element.is_a() == cls:
                held[(cls, element.GlobalId)] = (
                    getattr(element, "PredefinedType", None) or "-",
                    element.Name,
                    getattr(element, "Tag", None),
                )
    return held, collections.Counter(entity.is_a() for entity in model)


def main():
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    before, before_counts = load(sys.argv[1])
    after, after_counts = load(sys.argv[2])
    print(
        f"furniture elements and types: before {len(before)}, after {len(after)}; "
        f"only in before {len(set(before) - set(after))}, only in after {len(set(after) - set(before))}"
    )
    moves, other = collections.Counter(), 0
    for key in sorted(before.keys() & after.keys()):
        (was, name, tag), (now, new_name, new_tag) = before[key], after[key]
        if was != now:
            moves[(key[0], was, now)] += 1
        if (name, tag) != (new_name, new_tag):
            other += 1
    for (cls, was, now), count in moves.most_common():
        print(f"  {count:5}  {cls}: {was} -> {now}")
    print("name or tag differs on", other)
    delta = {k: after_counts[k] - before_counts[k] for k in set(after_counts) | set(before_counts) if after_counts[k] != before_counts[k]}
    print("entity count change (after minus before):", dict(sorted(delta.items())))
    return 0


if __name__ == "__main__":
    sys.exit(main())

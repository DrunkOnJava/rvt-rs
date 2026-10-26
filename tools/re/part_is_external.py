#!/usr/bin/env python3
"""Whether a member's or plate's IsExternal is its whole's (#35, W7).

revit-ifc's `CategoryUtil.IsElementExternal` (CLEANROOM.md source 5) gives a
family instance with no IsExternal parameter, and whose type has no
Function, its host's value. A curtain wall's mullions and panels are such
instances, and Revit's export aggregates them under the curtain wall.

Reads Revit's IFC export and the rvt-rs export next to the probe's output
(`model.ifc` in the probe output's directory; the probe's own output is
ignored). For each IfcMember and IfcPlate Revit writes IsExternal on, it
compares the value with the IsExternal of the element aggregating it, then
counts how many of those rvt-rs writes (by Tag) with the same whole, and
whether rvt-rs's whole has the same IsExternal.

Usage:

    python3 tools/re/part_is_external.py <probe.txt> <revit.ifc>
"""

import collections
import os
import sys

import ifcopenshell
import ifcopenshell.util.element

CLASSES = ("IfcMember", "IfcPlate")


def is_external(element):
    for props in ifcopenshell.util.element.get_psets(element, psets_only=True).values():
        if isinstance(props.get("IsExternal"), bool):
            return props["IsExternal"]
    return None


def wholes(f):
    out = {}
    for rel in f.by_type("IfcRelAggregates"):
        for part in rel.RelatedObjects:
            out[part.id()] = rel.RelatingObject
    return out


def main():
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    ours_path = os.path.join(os.path.dirname(os.path.abspath(args[0])), "model.ifc")
    revit = ifcopenshell.open(args[1])
    ours = ifcopenshell.open(ours_path) if os.path.exists(ours_path) else None
    revit_wholes = wholes(revit)
    ours_by_tag = {}
    ours_wholes = {}
    if ours is not None:
        ours_wholes = wholes(ours)
        for cls in CLASSES:
            for element in ours.by_type(cls):
                ours_by_tag.setdefault(element.Tag, element)
    tally = collections.Counter()
    examples = collections.defaultdict(list)
    for cls in CLASSES:
        for element in revit.by_type(cls):
            value = is_external(element)
            if value is None:
                continue
            whole = revit_wholes.get(element.id())
            if whole is None:
                key = (cls, "no whole", str(value))
            else:
                held = is_external(whole)
                verdict = "whole has none" if held is None else ("equal" if held == value else "different")
                key = (cls, whole.is_a(), verdict)
            tally[key] += 1
            if key[2] in ("different", "whole has none", "no whole") and len(examples[key]) < 5:
                examples[key].append(
                    f"{element.Tag} {element.Name!r} {value} whole "
                    + ("-" if whole is None else f"{whole.is_a()} {whole.Tag} {whole.Name!r} {is_external(whole)}")
                )
            if whole is None or ours is None:
                continue
            mine = ours_by_tag.get(element.Tag)
            if mine is None:
                tally[(cls, "rvt-rs", "not written")] += 1
                continue
            my_whole = ours_wholes.get(mine.id())
            if my_whole is None:
                tally[(cls, "rvt-rs", "no whole")] += 1
            elif my_whole.Tag != whole.Tag:
                tally[(cls, "rvt-rs", "another whole")] += 1
            else:
                held = is_external(my_whole)
                verdict = "whole has none" if held is None else ("whole equal" if held == value else "whole different")
                tally[(cls, "rvt-rs", verdict)] += 1
    for key in sorted(tally):
        print(f"  {key[0]:10} {key[1]:16} {key[2]:16} {tally[key]}")
    for key in sorted(examples):
        for line in examples[key]:
            print(f"  e.g. {key[0]} {key[1]} {key[2]}: {line}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

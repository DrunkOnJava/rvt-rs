#!/usr/bin/env python3
"""Find the element record reference-list slots that hold the element's
storey and type in Revit's export (RE-130, #228).

Input: the JSON lines of `examples/probe_re130_reference_slots.rs` (other
lines are skipped) and Revit's export of the same model.

For each record whose ElementId is an exported element's Tag, the element's
storey (IfcRelContainedInSpatialStructure) is mapped to its Level ElementId
through the Level name entries (Revit names a storey after its Level), and
its type (IfcRelDefinesByType) to the type object's Tag. Each is looked up
in the record's reference list and counted by category and slot, from the
start (`+i`) and from the end (`-i`). A record whose list lacks the id
counts as `absent`; one where it occurs twice counts every slot.

Usage:

    python3 tools/re/reference_slots_vs_ifc.py refs.jsonl revit-export.ifc [--list]

Needs IfcOpenShell (tested with 0.8.5).
"""

import collections
import json
import sys

import ifcopenshell


def main():
    args = sys.argv[1:]
    if len(args) < 2:
        raise SystemExit(__doc__)
    rows = [json.loads(line) for line in open(args[0]) if line.startswith("{")]
    levels = {}
    for row in rows:
        if "level" in row:
            levels.setdefault(row["name"], set()).add(row["level"])
    records = {}
    for row in rows:
        if "refs" in row:
            records.setdefault(row["id"], row)
    model = ifcopenshell.open(args[1])
    storey_of, type_of = {}, {}
    for rel in model.by_type("IfcRelContainedInSpatialStructure"):
        structure = rel.RelatingStructure
        if not structure.is_a("IfcBuildingStorey"):
            continue
        ids = levels.get(structure.Name)
        if not ids or len(ids) != 1:
            continue
        for element in rel.RelatedElements:
            tag = getattr(element, "Tag", None)
            if tag and str(tag).isdigit():
                storey_of[int(tag)] = next(iter(ids))
    for rel in model.by_type("IfcRelDefinesByType"):
        tag = getattr(rel.RelatingType, "Tag", None)
        if not tag or not str(tag).isdigit():
            continue
        for element in rel.RelatedObjects:
            own = getattr(element, "Tag", None)
            if own and str(own).isdigit():
                type_of[int(own)] = int(tag)
    counts = collections.Counter()
    for kind, table in (("storey", storey_of), ("type", type_of)):
        for tag, target in table.items():
            record = records.get(tag)
            if record is None:
                continue
            refs = record["refs"]
            slots = [i for i, value in enumerate(refs) if value == target]
            category = record["category"]
            if not slots:
                counts[(kind, category, "absent")] += 1
            for i in slots:
                counts[(kind, category, f"+{i} / -{len(refs) - i}")] += 1
    print(f"{len(levels)} level names, {len(records)} records, "
          f"{len(storey_of)} exported elements with a storey, {len(type_of)} with a type")
    for key, n in sorted(counts.items(), key=lambda kv: (kv[0][0], kv[0][1], -kv[1])):
        print(f"  {n:>5}  {key[0]} | {key[1]} | {key[2]}")


if __name__ == "__main__":
    main()

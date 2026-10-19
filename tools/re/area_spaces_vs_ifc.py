#!/usr/bin/env python3
"""Pair the Areas `probe_area_records` reads with Revit's `IfcSpace` (#763).

The probe prints one tab-separated `area` line per Area: ElementId, Revit
GlobalId, number, name, the one Level its records name, and whether its
placement word is RE-21's placed instance kind. Revit's exporter writes an
exported Area as an `IfcSpace` with the number as `Name` and the name as
`LongName`, in its Level's storey. This pairs each line with Revit's space of
the same GlobalId, counts the number, name and storey that agree, and prints
what Revit writes on those spaces (PredefinedType, ObjectType, body,
property and quantity sets) so a writer can match it.

Usage:
    python3 tools/re/area_spaces_vs_ifc.py PROBE.txt REVIT.ifc [--list]
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.util.element


def areas(path):
    out = []
    for line in open(path, encoding="utf-8", errors="replace"):
        parts = line.rstrip("\n").split("\t")
        if len(parts) == 7 and parts[0] == "area":
            _, element_id, global_id, number, name, level, placed = parts
            out.append((int(element_id), global_id, number, name, level, placed == "placed"))
    return out


def storey_of(space):
    parent = ifcopenshell.util.element.get_aggregate(space)
    while parent is not None and not parent.is_a("IfcBuildingStorey"):
        parent = ifcopenshell.util.element.get_aggregate(parent)
    return parent.Name if parent is not None else None


def body(space):
    if not space.Representation:
        return "none"
    kinds = []
    for representation in space.Representation.Representations:
        items = ",".join(sorted({item.is_a() for item in representation.Items}))
        kinds.append(f"{representation.RepresentationIdentifier}/{representation.RepresentationType}:{items}")
    return " ".join(sorted(kinds))


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    listing = "--list" in sys.argv
    probe, reference = args
    model = ifcopenshell.open(reference)
    spaces = {space.GlobalId: space for space in model.by_type("IfcSpace")}
    read = areas(probe)
    placed = [a for a in read if a[5]]
    print(f"Areas read: {len(read)}, placed instances {len(placed)}; Revit's export: {len(spaces)} IfcSpace")
    counts = collections.Counter()
    census = collections.defaultdict(collections.Counter)
    for element_id, global_id, number, name, level, is_placed in read:
        space = spaces.get(global_id)
        if space is None:
            counts["placed, not in Revit's export" if is_placed else "not placed, not in Revit's export"] += 1
            if listing:
                print(f"  {element_id} {global_id} '{number}' '{name}' {level}: no IfcSpace{'' if is_placed else ' (not placed)'}")
            continue
        counts["in Revit's export" + ("" if is_placed else ", not placed")] += 1
        storey = storey_of(space)
        same = (space.Name == number, space.LongName == name, storey == level)
        for label, ok in zip(("number", "name", "storey"), same):
            counts[f"{label} equal" if ok else f"{label} differs"] += 1
        if listing and not all(same):
            print(f"  {element_id} {global_id}: ours '{number}' '{name}' {level} | Revit '{space.Name}' '{space.LongName}' {storey}")
        census["PredefinedType"][str(space.PredefinedType)] += 1
        census["ObjectType"][str(space.ObjectType)] += 1
        census["CompositionType"][str(space.CompositionType)] += 1
        census["body"][body(space)] += 1
        space_type = ifcopenshell.util.element.get_type(space)
        census["type object"][
            "none" if space_type is None else f"{space_type.is_a()} {space_type.PredefinedType}"
        ] += 1
        if space_type is not None and listing and counts["type shown"] < 3:
            counts["type shown"] += 1
            print(f"  {element_id} type: {space_type.is_a()} Name '{space_type.Name}' Tag '{space_type.Tag}'")
        for set_name, values in ifcopenshell.util.element.get_psets(space).items():
            census["property set"][set_name] += 1
            for key, value in values.items():
                if key != "id":
                    census[f"{set_name}"][key] += 1
                    census[f"{set_name}.{key} values"][str(value)[:40]] += 1
    for key in sorted(counts):
        if key != "type shown":
            print(f"  {counts[key]:>5}  {key}")
    print("what Revit writes on the paired spaces:")
    shown = ("PredefinedType", "ObjectType", "CompositionType", "body", "type object", "property set")
    for field in shown:
        print(f"  {field}: " + ", ".join(f"{v} {n}" for v, n in census[field].most_common()))
    for field in sorted(census):
        if field in shown:
            continue
        print(f"  {field}: " + ", ".join(f"{v} {n}" for v, n in census[field].most_common(8)))


if __name__ == "__main__":
    main()

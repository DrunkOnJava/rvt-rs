#!/usr/bin/env python3
"""Name the element behind each IfcSpace of Revit's export, by GlobalId (#763).

Reads the output of `examples/probe_global_id_categories.rs` (each declared
ElementId, the GlobalId rvt-rs rebuilds for it, and its element records'
categories) and Revit's IFC export of the same file. Every IfcSpace of
Revit's export is looked up by GlobalId: the counts say how many are rooms,
areas, MEP spaces, family instances of some category, elements without a
record, or GlobalIds no declared element has. `--list` lists every space with
its Name, LongName, storey, ElementId and records.

Usage:

    python3 tools/re/global_id_categories_vs_ifc.py <probe output> <revit-export.ifc> [--list]

Needs IfcOpenShell (tested with 0.9.0.post1).
"""

import collections
import re
import sys

import ifcopenshell
import ifcopenshell.util.element

LINE = re.compile(r"^  (\d+) (\S+) (\S+)$")


def storey(product):
    """The name of the storey a product is contained in or part of, or "-"."""
    seen = 0
    while product is not None and seen < 16:
        if product.is_a("IfcBuildingStorey"):
            return product.Name or "-"
        product = ifcopenshell.util.element.get_container(product) or ifcopenshell.util.element.get_aggregate(product)
        seen += 1
    return "-"


def main(argv):
    listing = "--list" in argv
    argv = [a for a in argv if a != "--list"]
    if len(argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    by_global_id = {}
    for line in open(argv[0], errors="replace"):
        m = LINE.match(line.rstrip("\n"))
        if m and m.group(2) != "-":
            by_global_id[m.group(2)] = (m.group(1), m.group(3))
    model = ifcopenshell.open(argv[1])
    spaces = model.by_type("IfcSpace")
    kinds = collections.Counter()
    rows = []
    for space in spaces:
        found = by_global_id.get(space.GlobalId)
        if found is None:
            kind, element_id, records = "GlobalId of no declared element", "-", "-"
        else:
            element_id, records = found
            kind = "no element record" if records == "-" else " + ".join(sorted({r.split("|")[0] for r in records.split(";")}))
        kinds[kind] += 1
        rows.append((kind, space.Name or "-", space.LongName or "-", storey(space), element_id, records, space.GlobalId))
    print(f"Revit's export: {len(spaces)} IfcSpace; probe: {len(by_global_id)} GlobalIds")
    print("IfcSpace by the category of the element behind it:")
    for kind, n in kinds.most_common():
        print(f"  {n:6}  {kind}")
    by_kind = collections.defaultdict(collections.Counter)
    for kind, name, long_name, level, *_ in rows:
        by_kind[kind][long_name if long_name != "-" else name.split(":")[0]] += 1
    print("their LongNames (or family names), per category:")
    for kind in sorted(by_kind):
        names = ", ".join(f"{n!r} {c}" for n, c in by_kind[kind].most_common(12))
        print(f"  {kind}: {names}")
    if listing:
        print("spaces:")
        for kind, name, long_name, level, element_id, records, global_id in sorted(rows):
            print(f"  {global_id}  {name!r}  {long_name!r}  storey {level!r}  id {element_id}  {records}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

#!/usr/bin/env python3
"""Score placed-instance ElementIds against a Revit IFC export's Tags (RE-124).

Reads the `category <id> <name> <count> <ids>` lines that
`examples/probe_re124_revit_2026_records.rs` prints and compares the ids with
the numeric `Tag` of every element of the export, openings and virtual
elements left out. Prints true and false positives, misses by IFC class, and
the IFC class each found category lands on.

Usage:

    python3 tools/re/instances_vs_ifc_tags.py <probe-output.txt> <revit-export.ifc>
"""

import collections
import sys

import ifcopenshell


def main(argv):
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    ours = {}
    for line in open(argv[1]):
        if line.startswith("category "):
            _, _, name, _count, ids = line.rstrip("\n").split(" ", 4)
            for element in filter(None, ids.split(",")):
                ours[int(element)] = name
    tags = {}
    for element in ifcopenshell.open(argv[2]).by_type("IfcElement"):
        if element.is_a("IfcOpeningElement") or element.is_a("IfcVirtualElement"):
            continue
        tag = getattr(element, "Tag", None)
        if tag and tag.isdigit():
            tags[int(tag)] = element.is_a()
    found = set(ours) & set(tags)
    print(f"export elements with a numeric Tag: {len(tags)}")
    print(f"placed instances: {len(ours)}; in the export {len(found)}, not in it {len(set(ours) - set(tags))}, missed {len(set(tags) - set(ours))}")
    print("missed, by IFC class:", dict(collections.Counter(tags[i] for i in set(tags) - set(ours))))
    for (name, cls), n in sorted(collections.Counter((ours[i], tags[i]) for i in found).items()):
        print(f"  {name} -> {cls}: {n}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

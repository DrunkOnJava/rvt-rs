#!/usr/bin/env python3
"""Check RE-84's void-only rule against Revit's own IFC4 export.

Research tool for `reports/element-framing/RE-84-void-only-doors-windows.md`
(#309, #227). It reads the output of
`examples/probe_re84_void_only_families.rs` (one door or window record per
line: ElementId, class, type ElementId, how many map materials the type has,
family) and finds each ElementId in Revit's export:

- `element`: an element with that Tag (an `IfcDoor`, `IfcWindow`, ...);
- `opening only`: no element, but an `IfcOpeningElement` with that Tag and
  nothing filling it;
- `absent`: neither.

It prints the count of each outcome for types with a map (`n`), types with a
value block and no map (`0`), types with no value block (`-`) and elements
with no type (`?`).

Usage:

    cargo run --profile ci --example probe_re84_void_only_families -- model.rvt > probe.tsv
    python3 tools/re/void_only_vs_ifc.py probe.tsv <revit-export.ifc> [--list]

Needs IfcOpenShell (tested with 0.8.5).
"""

import collections
import sys

import ifcopenshell


def main(argv):
    args = argv[1:]
    listing = "--list" in args
    args = [a for a in args if a != "--list"]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    f = ifcopenshell.open(args[1])
    filled = {r.RelatingOpeningElement.id() for r in f.by_type("IfcRelFillsElement")}
    elements = {
        e.Tag
        for e in f.by_type("IfcElement")
        if e.Tag and not e.is_a("IfcOpeningElement")
    }
    unfilled = {
        o.Tag for o in f.by_type("IfcOpeningElement") if o.Tag and o.id() not in filled
    }
    table = collections.Counter()
    rows = []
    for line in open(args[0], encoding="utf-8"):
        parts = line.rstrip("\n").split("\t")
        if len(parts) < 5 or not parts[0]:
            continue
        tag, cls, _type_id, materials, family = parts[:5]
        if materials not in ("0", "-", "?"):
            materials = "n"
        if tag in elements:
            outcome = "element"
        elif tag in unfilled:
            outcome = "opening only"
        else:
            outcome = "absent"
        table[(materials, outcome)] += 1
        rows.append((materials, outcome, tag, cls, family))
    for materials in ("n", "0", "-", "?"):
        counts = {o: table[(materials, o)] for o in ("element", "opening only", "absent")}
        if any(counts.values()):
            print(f"type map materials {materials}: {counts}")
    if listing:
        for row in sorted(rows):
            if row[0] != "n" or row[1] != "element":
                print("  ", *row)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

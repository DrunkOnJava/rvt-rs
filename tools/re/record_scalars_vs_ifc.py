#!/usr/bin/env python3
"""Which native record field tells a boolean property Revit's export writes
(RE-228, #35, W7).

Reads a probe's `rec <tag> <record class> <object>.<field>=<value> ...`
lines (`examples/probe_re228_wall_record_scalars.rs`) and Revit's IFC
export. For every boolean property of every property set Revit writes on
those elements (by Tag), and every field the probe printed, it takes the
one value of the field whose presence best predicts the property (the
property is true exactly where the field has that value, or exactly where
it has not) and counts how many elements that gets right. A field missing
from a record counts as the value `-`.

It prints, per property, the true and false counts and the 12 best fields
with their accuracy, perfect ones first.

Usage:

    python3 tools/re/record_scalars_vs_ifc.py <probe.txt> <revit.ifc> [--list]
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.util.element


def read_probe(path):
    rows = {}
    for line in open(path, encoding="utf-8", errors="replace"):
        parts = line.split()
        if len(parts) < 3 or parts[0] != "rec" or parts[2] == "-":
            continue
        fields = {}
        for part in parts[3:]:
            name, _, value = part.partition("=")
            fields[name] = value
        rows[parts[1]] = fields
    return rows


def main():
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    rows = read_probe(args[0])
    revit = ifcopenshell.open(args[1])
    truths = collections.defaultdict(dict)
    for element in revit.by_type("IfcElement"):
        tag = str(element.Tag) if element.Tag else None
        if tag not in rows:
            continue
        for pset, props in ifcopenshell.util.element.get_psets(element, psets_only=True).items():
            for prop, value in props.items():
                if isinstance(value, bool):
                    truths[f"{pset}.{prop}"][tag] = value
    print(f"probe: {len(rows)} records; boolean properties Revit writes on them: {len(truths)}")
    names = sorted({name for fields in rows.values() for name in fields})
    for prop, by_tag in sorted(truths.items()):
        tags = sorted(by_tag)
        trues = sum(by_tag[t] for t in tags)
        print(f"{prop}: {len(tags)} elements, true {trues}, false {len(tags) - trues}")
        if trues in (0, len(tags)):
            print("  constant: no field needed")
            continue
        ranked = []
        for name in names:
            values = collections.Counter(rows[t].get(name, "-") for t in tags)
            if len(values) < 2:
                continue
            best = None
            for value in values:
                hits = sum((rows[t].get(name, "-") == value) == by_tag[t] for t in tags)
                score = max(hits, len(tags) - hits)
                sense = "=" if hits >= len(tags) - hits else "!="
                if best is None or score > best[0]:
                    best = (score, sense, value)
            ranked.append((best[0], name, best[1], best[2], len(values)))
        ranked.sort(key=lambda r: (-r[0], r[1]))
        for score, name, sense, value, distinct in ranked[:12]:
            print(f"  {score}/{len(tags)}  true where {name} {sense} {value}  ({distinct} values)")
    if "--list" in sys.argv:
        for prop, by_tag in sorted(truths.items()):
            for tag in sorted(by_tag)[:40]:
                print(f"row {prop} {tag} {by_tag[tag]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

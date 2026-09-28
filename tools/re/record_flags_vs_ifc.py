#!/usr/bin/env python3
"""Tabulate element records' `+0x46` flag word against Revit's export (RE-128, #223).

Input: the JSON lines of `examples/probe_re128_record_flags.rs` (other lines
are skipped) and Revit's export of the same model.

Each record is joined to Revit's export by Tag (its ElementId). For each
distinct (category, class, flags) the table counts the records Revit
exports and those it does not, and, of the exported ones, how many Revit's
property sets call IsExternal and LoadBearing true or false. A second
table does the same per bit, for the bits that are not the same on every
record.

Usage:

    python3 tools/re/record_flags_vs_ifc.py records.jsonl revit-export.ifc [--list]

Needs IfcOpenShell (tested with 0.8.5).
"""

import collections
import json
import sys

import ifcopenshell
import ifcopenshell.util.element


def main():
    args = sys.argv[1:]
    if len(args) < 2:
        raise SystemExit(__doc__)
    records = [json.loads(line) for line in open(args[0]) if line.startswith("{")]
    model = ifcopenshell.open(args[1])
    exported = {}
    for element in model.by_type("IfcElement"):
        tag = getattr(element, "Tag", None)
        if not tag or not str(tag).isdigit():
            continue
        values = {}
        for pset in ifcopenshell.util.element.get_psets(element).values():
            for name in ("IsExternal", "LoadBearing"):
                if isinstance(pset.get(name), bool):
                    values.setdefault(name, pset[name])
        exported[int(tag)] = (element.is_a(), values)

    def row():
        return collections.Counter()

    by_value = collections.defaultdict(row)
    by_bit = collections.defaultdict(row)
    varying = 0
    for record in records:
        varying |= record["flags"] ^ records[0]["flags"]
    for record in records:
        key = (record["category"], record["class"], record["flags"])
        hit = exported.get(record["id"])
        counts = [by_value[key]]
        for bit in range(32):
            if varying >> bit & 1:
                counts.append(by_bit[(bit, record["flags"] >> bit & 1)])
        for c in counts:
            c["records"] += 1
            if hit is None:
                c["not exported"] += 1
                continue
            c["exported"] += 1
            for name, value in hit[1].items():
                c[f"{name}={'T' if value else 'F'}"] += 1
    columns = ["records", "exported", "not exported", "IsExternal=T", "IsExternal=F", "LoadBearing=T", "LoadBearing=F"]
    print(f"{len(records)} records, {sum(1 for r in records if r['id'] in exported)} in Revit's export")
    print("category | class | flags | " + " | ".join(columns))
    for (category, cls, flags), c in sorted(by_value.items(), key=lambda kv: -kv[1]["records"]):
        print(f"{category} | {cls} | {flags:#010x} | " + " | ".join(str(c[k]) for k in columns))
    print()
    print("bit | value | " + " | ".join(columns))
    for (bit, value), c in sorted(by_bit.items()):
        print(f"{bit} | {value} | " + " | ".join(str(c[k]) for k in columns))


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Read `probe_re326_phase_census` against Revit's export (RE-326).

The probe prints one `group` line per (class, created phase, demolished
phase) with the ElementIds of its records. Revit's exporter writes an
element's ElementId as the `Tag` of its IfcElement, so this counts, for each
group, how many of its ElementIds are the Tag of an element of Revit's export.

A group created in the last phase that is in Revit's export means the export
phase was the last; one that is absent means it was an earlier phase. A group
demolished in the last phase and in Revit's export means the same the other
way round.

Usage:
    python3 tools/re/phase_census_vs_ifc.py PROBE.txt REVIT.ifc [--classes]
"""

import collections
import json
import sys

import ifcopenshell


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    by_class = "--classes" in sys.argv
    probe, reference = args
    model = ifcopenshell.open(reference)
    tags = {e.Tag for e in model.by_type("IfcElement") if e.Tag}
    print(f"Revit's export: {len(tags)} IfcElement Tags")
    info = None
    totals = collections.defaultdict(lambda: [0, 0])
    for line in open(probe, encoding="utf-8", errors="replace"):
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            record = json.loads(line)
        except ValueError:
            continue
        if record.get("kind") == "file":
            info = record
            names = {p[0]: p[1] for p in info["phase_order"]}
            print(f"{info['file']}: Revit {info['revit']}, phases {info['phase_order']}")
        elif record.get("kind") == "group" and info is not None:
            held = sum(str(i) in tags for i in record["ids"])
            key = (
                names.get(record["created"], record["created"]),
                names.get(record["demolished"], "-" if record["demolished"] == -1 else record["demolished"]),
            )
            totals[key][0] += record["records"]
            totals[key][1] += held
            if by_class:
                print(
                    f"  {record['class']}: created {key[0]}, demolished {key[1]}: "
                    f"{record['records']} records, {held} in Revit's export"
                )
    for (created, demolished), (records, held) in sorted(totals.items(), key=str):
        print(f"created {created}, demolished {demolished}: {records} records, {held} are the Tag of an element of Revit's export")


if __name__ == "__main__":
    main()

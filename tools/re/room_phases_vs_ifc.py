#!/usr/bin/env python3
"""Pair the rooms `probe_re326_room_phases` reads with Revit's `IfcSpace` (RE-326).

The probe prints one JSON line per file and one per `RoomElem` record: its
created phase, every ElementId in its data that is one of the project's
phases (by field path), every ElementId-storage saved parameter (by built-in
id), what rvt-rs writes it as (`room`, `area` or `other`) and Revit's
GlobalId for it. Revit's exporter leaves out a room whose own phase
(`ROOM_PHASE`) is not the export phase, the last of the project's phases
(revit-ifc, `ElementFilteringUtil.IsRoomInInvalidPhase`).

For each candidate source of a record's phase (the created phase, each
phase-valued field, each ElementId parameter) this tabulates the records by
whether the source names the export phase, another phase or nothing, and by
whether Revit's export holds an `IfcSpace` with the record's GlobalId. A
source that is a room's phase shows no record in another phase that Revit's
export holds, and every record in the export phase that Revit's export holds
or leaves out for another reason (unplaced, not enclosed).

Usage:
    python3 tools/re/room_phases_vs_ifc.py PROBE.txt REVIT.ifc [--list]
"""

import collections
import json
import sys

import ifcopenshell


def read_probe(path):
    files = []
    for line in open(path, encoding="utf-8", errors="replace"):
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            record = json.loads(line)
        except ValueError:
            continue
        if record.get("kind") == "file":
            files.append({"info": record, "records": []})
        elif files:
            files[-1]["records"].append(record)
    return files


def sources(record):
    """The candidate phase of a record, by source name."""
    out = {"created": record.get("created")}
    for path, value in record.get("phase_refs", []):
        out[f"field {path}"] = value
    for entry in record.get("element_id_parameters", []):
        parameter, value = entry[0], entry[1]
        out[f"parameter {parameter}"] = value
    return out


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    listing = "--list" in sys.argv
    probe, reference = args
    model = ifcopenshell.open(reference)
    spaces = model.by_type("IfcSpace")
    revit_ids = {space.GlobalId for space in spaces}
    print(f"Revit's export: {len(spaces)} IfcSpace")
    for entry in read_probe(probe):
        info, records = entry["info"], entry["records"]
        export = info.get("export_phase")
        order = [phase[0] for phase in info.get("phase_order", [])]
        names = {phase[0]: phase[1] for phase in info.get("phase_order", [])}
        print(
            f"{info['file']}: Revit {info['revit']}, phases "
            f"{[(p, names[p]) for p in order]}, export phase {export}; "
            f"RoomElem records {info['room_elem_records']}, rvt-rs writes "
            f"{info['written_rooms']} rooms and {info['written_areas']} areas; "
            f"census {info.get('spatial_census')}"
        )
        if info.get("written_error"):
            print(f"  rvt-rs read failed: {info['written_error']}")
        by_kind = collections.Counter(r["kind"] for r in records)
        print(f"  records by kind: {dict(by_kind)}")
        held = collections.Counter(
            r["kind"] for r in records if r.get("global_id") in revit_ids
        )
        print(f"  held by Revit's export (GlobalId): {dict(held)}")
        for kind in ("room", "area", "other"):
            subset = [r for r in records if r["kind"] == kind and "created" in r]
            if not subset:
                continue
            table = collections.defaultdict(collections.Counter)
            everywhere = collections.Counter()
            for record in subset:
                for source, value in sources(record).items():
                    everywhere[source] += 1
                    if value is None or value == -1:
                        state = "unset"
                    elif value == export:
                        state = "export phase"
                    elif value in names:
                        state = "other phase"
                    else:
                        state = "not a phase"
                    in_revit = (
                        "in Revit" if record.get("global_id") in revit_ids else "not in Revit"
                    )
                    table[source][(state, in_revit)] += 1
            print(f"  {kind}: {len(subset)} records")
            for source in sorted(table, key=lambda s: (s != "created", s)):
                cells = table[source]
                row = ", ".join(
                    f"{state} / {where} {count}" for (state, where), count in sorted(cells.items())
                )
                print(f"    {source} (on {everywhere[source]}): {row}")
            if listing:
                for record in subset:
                    other = [
                        s
                        for s, v in sources(record).items()
                        if v not in (None, -1, export) and v in names
                    ]
                    if other:
                        where = "in Revit" if record.get("global_id") in revit_ids else "not in Revit"
                        print(
                            f"    {record['id']} '{record.get('number')}' '{record.get('name')}': "
                            f"not in the export phase by {other}; {where}"
                        )


if __name__ == "__main__":
    main()

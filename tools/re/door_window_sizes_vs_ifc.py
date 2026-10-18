#!/usr/bin/env python3
"""Score each candidate size of a door or window type against Revit's IFC.

Research tool for RE-187 (#759). `examples/probe_re187_door_window_sizes.rs`
prints, per door and window, the sizes its type holds: the BuiltInParameters
Width and Height (-1001301, -1001300), Rough Width and Rough Height
(-1001305, -1001304), and family parameters named Width and Height. This
scorer joins those rows to Revit's own export by `Tag` (the ElementId) and
counts, per class, how many of Revit's `OverallWidth` and `OverallHeight`
each candidate pair equals, within 0.001 ft.

Usage:

    python3 tools/re/door_window_sizes_vs_ifc.py <probe output> <revit-export.ifc> [--list]

Needs IfcOpenShell (tested with 0.8.5).
"""

import collections
import json
import sys

import ifcopenshell
import ifcopenshell.util.unit

FEET = 0.3048
TOLERANCE_FEET = 1e-3
CANDIDATES = [
    ("width/height (-1001301/-1001300)", "width", "height"),
    ("rough width/height (-1001305/-1001304)", "rough_width", "rough_height"),
    ("family Width/Height", "family_width", "family_height"),
]


def revit_sizes(path):
    """Tag -> (class, OverallWidth ft, OverallHeight ft) for every door and window."""
    f = ifcopenshell.open(path)
    metres = ifcopenshell.util.unit.calculate_unit_scale(f)
    out = {}
    for entity in ("IfcDoor", "IfcWindow"):
        for element in f.by_type(entity):
            if not element.Tag:
                continue
            width, height = element.OverallWidth, element.OverallHeight
            out[str(element.Tag)] = (
                entity,
                None if width is None else width * metres / FEET,
                None if height is None else height * metres / FEET,
            )
    return out


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    listing = "--list" in sys.argv
    probe_path, ifc_path = args[0], args[1]
    rows = []
    for line in open(probe_path):
        line = line.strip()
        if line.startswith("{"):
            row = json.loads(line)
            if row.get("id") is not None:
                rows.append(row)
    revit = revit_sizes(ifc_path)
    with_size = {tag: v for tag, v in revit.items() if v[1] is not None and v[2] is not None}
    print(f"Revit's export: {len(revit)} doors and windows, {len(with_size)} with OverallWidth and OverallHeight")
    print(f"probe: {len(rows)} doors and windows from element records")

    tally = collections.Counter()
    seen = collections.Counter()
    worst = []
    for row in rows:
        tag = str(row["id"])
        if tag not in with_size:
            continue
        entity, width, height = with_size[tag]
        seen[entity] += 1
        matched = []
        for label, wkey, hkey in CANDIDATES:
            w, h = row.get(wkey), row.get(hkey)
            if w is None or h is None:
                tally[(entity, label, "not read")] += 1
                continue
            if abs(w - width) <= TOLERANCE_FEET and abs(h - height) <= TOLERANCE_FEET:
                tally[(entity, label, "equal")] += 1
                matched.append(label)
            else:
                tally[(entity, label, "differs")] += 1
        if not matched:
            worst.append((tag, entity, width, height, row))

    for entity in sorted(seen):
        print(f"\n{entity}: {seen[entity]} matched by Tag")
        for label, _, _ in CANDIDATES:
            equal = tally[(entity, label, "equal")]
            differs = tally[(entity, label, "differs")]
            missing = tally[(entity, label, "not read")]
            print(f"  {label:42} equal {equal:4}  differs {differs:4}  not read {missing:4}")
    if listing:
        print(f"\n{len(worst)} with no candidate equal to Revit's size:")
        for tag, entity, width, height, row in worst[:60]:
            cands = ", ".join(
                f"{label.split(' (')[0]}={row.get(w)}x{row.get(h)}" for label, w, h in CANDIDATES
            )
            print(f"  {entity} {tag} type {row.get('type')}: Revit {width:.4f} x {height:.4f} ft; {cands}; box {row.get('box')}")


main()

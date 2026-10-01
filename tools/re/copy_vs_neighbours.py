#!/usr/bin/env python3
"""Which copy of a record the neighbouring releases of the same model agree with (#548).

Research tool for `reports/element-framing/RE-143-which-copy-is-current.md`. A project whose
partitions hold two records for one ElementId (cross-partition copies, #548) has no Revit export
to say which is current. Autodesk re-saves its sample projects in every release, so the same
element appears once in the release before and the release after. For every id with two chain
records whose boxes differ, both records carrying the element marker, this counts which copy
(the one listed first, in the first partition, or the second) each neighbouring release's single
record agrees with, to 0.001 ft.

The input is the `measure-head` artifact of a Measure run with `-f samples=true
-f probe=probe_re135_undeclared_records` (it holds `sample-<year>-<file>/probe.txt` with the
probe's `R id class category M|- box` lines).

Usage:

    python3 tools/re/copy_vs_neighbours.py <measure-head dir> <rac_basic|rst_basic|rac_advanced> <year> <neighbour year> ...
"""

import json
import os
import sys

NAMES = {
    "rac_basic": ["rac_basic_sample_project", "racbasicsampleproject"],
    "rst_basic": ["rst_basic_sample_project", "rstbasicsampleproject"],
    "rac_advanced": ["rac_advanced_sample_project", "racadvancedsampleproject"],
}
TOLERANCE = 1e-3


def probe_path(root, model, year):
    for name in NAMES[model]:
        path = os.path.join(root, f"sample-{year}-{name}", "probe.txt")
        if os.path.exists(path):
            return path
    return None


def load(path):
    """Every chain record by ElementId: (class, category, marked, box), and the partitions listed."""
    records, streams = {}, []
    for line in open(path, errors="replace"):
        if line.startswith("R "):
            parts = line.split()
            if len(parts) >= 11:
                box = tuple(float(x) for x in parts[5:11])
                records.setdefault(int(parts[1]), []).append((parts[2], parts[3], parts[4] == "M", box))
        elif line.startswith('{"revit"') and '"header_tag"' in line:
            streams = [(s["stream"], s["in_chain"]) for s in json.loads(line)["streams"]]
    return records, streams


def same(a, b):
    return all(abs(x - y) <= TOLERANCE for x, y in zip(a, b))


def main():
    if len(sys.argv) < 5 or sys.argv[2] not in NAMES:
        sys.exit(__doc__)
    root, model, year = sys.argv[1], sys.argv[2], int(sys.argv[3])
    neighbours = [int(y) for y in sys.argv[4:]]
    path = probe_path(root, model, year)
    if path is None:
        sys.exit(f"no probe output for {model} {year} under {root}")
    mine, streams = load(path)
    copies = {i: v for i, v in mine.items() if len(v) > 1}
    print(f"{model} {year}: partitions (records in the chain) {streams}")
    print(f"{len(copies)} ids have two or more chain records")
    for other_year in neighbours:
        other_path = probe_path(root, model, other_year)
        other, _ = load(other_path) if other_path else ({}, [])
        counts = {"unmarked": 0, "identical boxes": 0, "boxes differ": 0, "no single marked record in the neighbour": 0}
        agrees = {"first copy": 0, "second copy": 0, "both": 0, "neither": 0}
        for i, recs in copies.items():
            first, second = recs[0], recs[1]
            if not (first[2] and second[2]):
                counts["unmarked"] += 1
            elif same(first[3], second[3]):
                counts["identical boxes"] += 1
            else:
                counts["boxes differ"] += 1
                theirs = other.get(i, [])
                if len(theirs) != 1 or not theirs[0][2]:
                    counts["no single marked record in the neighbour"] += 1
                    continue
                a, b = same(first[3], theirs[0][3]), same(second[3], theirs[0][3])
                agrees["both" if a and b else "first copy" if a else "second copy" if b else "neither"] += 1
        print(f"  against {other_year}: {counts}")
        print(f"    of the ids whose boxes differ, the neighbour agrees with: {agrees}")


if __name__ == "__main__":
    main()

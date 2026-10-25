#!/usr/bin/env python3
"""Which stored length of a door's or window's symbol is the
`OverallWidth` or `OverallHeight` Revit's export writes (RE-229, #810).

Reads `examples/probe_re229_symbol_lengths.rs`'s output and Revit's IFC
export. For each parameter (id and name), over the doors and windows whose
symbol stores it, it counts how often its value is Revit's width and how
often Revit's height, within 0.001 ft. Then, for the doors and windows
whose width no parameter gives, Revit's sizes and every length the symbol
stores.

Usage:

    python3 tools/re/symbol_lengths_vs_ifc.py <probe.txt> <revit.ifc> [--list]
"""

import collections
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from door_window_sizes_vs_ifc import TOLERANCE_FEET, revit_sizes  # noqa: E402


def read_probe(path):
    rows = {}
    for line in open(path, encoding="utf-8", errors="replace"):
        parts = line.split()
        if len(parts) < 4 or parts[0] != "len" or parts[3] == "-":
            continue
        lengths = {}
        for part in parts[4:]:
            key, _, value = part.rpartition("=")
            lengths[key] = float(value)
        rows[parts[1]] = (parts[2], parts[3], lengths)
    return rows


def main():
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    rows = read_probe(args[0])
    revit = revit_sizes(args[1])
    tally = collections.defaultdict(collections.Counter)
    unexplained = []
    for tag, (cls, symbol, lengths) in rows.items():
        if tag not in revit or revit[tag][1] is None:
            continue
        _, width, height = revit[tag]
        hit = False
        for key, value in lengths.items():
            t = tally[(cls, key)]
            t["n"] += 1
            if abs(value - width) <= TOLERANCE_FEET:
                t["width"] += 1
                hit = True
            if abs(value - height) <= TOLERANCE_FEET:
                t["height"] += 1
        if not hit:
            unexplained.append((tag, cls, symbol, width, height, lengths))
    print(f"probe: {len(rows)} doors and windows with a symbol; Revit's export: {len(revit)}")
    print("parameter (class, id:name): elements storing it, = Revit width, = Revit height")
    ranked = sorted(tally.items(), key=lambda kv: -(kv[1]["width"] + kv[1]["height"]))
    for (cls, key), t in ranked[:30]:
        if t["width"] or t["height"]:
            print(f"  {cls:<9} {key:<40} {t['n']:>5} {t['width']:>5} {t['height']:>5}")
    print(f"no stored length is Revit's width: {len(unexplained)}")
    for tag, cls, symbol, width, height, lengths in unexplained[:30]:
        text = " ".join(f"{k}={v:.4f}" for k, v in sorted(lengths.items()))
        print(f"  {tag} {cls} symbol {symbol} Revit {width:.4f}x{height:.4f}: {text}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

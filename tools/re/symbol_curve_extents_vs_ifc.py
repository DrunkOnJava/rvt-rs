#!/usr/bin/env python3
"""Does the extent along X of the lines of no category in a door's or
window's symbol graphics give Revit's `OverallWidth` (RE-318, #810)?

Reads `probes/examples/probe_re318_symbol_curve_extents.rs`'s output and
Revit's IFC export. For each door and window the probe lists, joined to
Revit's by Tag, it compares Revit's `OverallWidth`, within 0.001 ft, with the
extent along the symbol's X of:

- wire: the `GLine`s of category -1 whose visibility bits (0x1e0 of
  `m_GInfo.m_flags`) are clear, the edges of the boxes that bound the
  symbol's forms;
- wire, any flags: the same, hidden ones included;
- every curve (`GLine` and `GArc`) whose visibility bits are clear.

A door or window Revit gives no width, or 0, is counted apart; the rule
should give such a symbol no lines. Prints every one `wire` misses.

Usage:

    python3 tools/re/symbol_curve_extents_vs_ifc.py <probe.txt> <revit.ifc> [--list]
"""

import collections
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from door_window_sizes_vs_ifc import TOLERANCE_FEET, revit_sizes  # noqa: E402

VISIBILITY_BITS = 0x1E0


def read_probe(path):
    instances, curves = {}, collections.defaultdict(list)
    for line in open(path, encoding="utf-8", errors="replace"):
        parts = line.split()
        if len(parts) >= 4 and parts[0] == "inst" and parts[3].startswith("symbol="):
            instances[parts[1]] = (parts[2], parts[3].split("=", 1)[1])
        elif len(parts) >= 8 and parts[0] == "cext":
            fields = dict(p.split("=", 1) for p in parts[3:] if "=" in p)
            curves[parts[1]].append(
                (
                    parts[2],
                    int(fields["cat"]),
                    int(fields["flags"]),
                    float(fields["x0"]),
                    float(fields["x1"]),
                )
            )
    return instances, curves


def extent(ranges):
    ranges = list(ranges)
    if not ranges:
        return None
    return max(r[1] for r in ranges) - min(r[0] for r in ranges)


def main():
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    instances, curves = read_probe(args[0])
    revit = revit_sizes(args[1])
    for tag, (cls, width, height) in sorted(revit.items(), key=lambda kv: (len(kv[0]), kv[0])):
        if tag in instances:
            w = "-" if width is None else f"{width:.6f}"
            h = "-" if height is None else f"{height:.6f}"
            print(f"revit {tag} {cls} {w} {h}")
    print(f"probe: {len(instances)} doors and windows; Revit's export: {len(revit)}")
    counts = collections.defaultdict(collections.Counter)
    misses = []
    for tag, (cls, symbol) in sorted(instances.items(), key=lambda kv: (len(kv[0]), kv[0])):
        if tag not in revit:
            continue
        _, width, _ = revit[tag]
        rows = curves.get(symbol, [])
        wire = extent((x0, x1) for k, cat, fl, x0, x1 in rows if k == "GLine" and cat == -1 and not fl & VISIBILITY_BITS)
        if width is None or width == 0.0:
            counts[cls]["Revit gives no width"] += 1
            counts[cls]["... and the symbol has no wire"] += wire is None
            continue
        variants = {
            "wire": wire,
            "wire, any flags": extent((x0, x1) for k, cat, fl, x0, x1 in rows if k == "GLine" and cat == -1),
            "every curve, visible": extent((x0, x1) for k, cat, fl, x0, x1 in rows if not fl & VISIBILITY_BITS),
        }
        counts[cls]["joined"] += 1
        for name, value in variants.items():
            counts[cls][name] += value is not None and abs(value - width) <= TOLERANCE_FEET
        if wire is None or abs(wire - width) > TOLERANCE_FEET:
            misses.append((tag, cls, symbol, width, wire))
    for cls, c in sorted(counts.items()):
        print(cls + ": " + ", ".join(f"{k} {v}" for k, v in c.items()))
    print(f"wire misses Revit's width on {len(misses)}")
    for m in misses[:40]:
        print("  ", m)
    return 0


if __name__ == "__main__":
    sys.exit(main())

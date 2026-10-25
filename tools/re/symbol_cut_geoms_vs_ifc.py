#!/usr/bin/env python3
"""Does the solid a family symbol cuts (`m_complexCutGeoms`) give the
`OverallWidth` and `OverallHeight` Revit's export writes where the symbol
has no cut loop (RE-225, #810, #824)?

Reads `examples/probe_re225_symbol_cut_geoms.rs`'s output and Revit's IFC
export. For each door and window the probe lists, joined to Revit's by
Tag, it compares Revit's two sizes, within 0.001 ft, with:

- the RE-215 cut loop's width and height, where the symbol has one;
- the meshed cut geometry's extent along the symbol's X and Z, and along X
  and Y (revit-ifc measures a family no wall hosts along Y), where the
  probe meshed any.

Usage:

    python3 tools/re/symbol_cut_geoms_vs_ifc.py <probe.txt> <revit.ifc> [--list]
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
        if len(parts) < 9 or parts[0] != "cut":
            continue
        fields = dict(p.split("=", 1) for p in parts[4:] if "=" in p)
        loop = None
        if fields.get("loop", "-") != "-":
            w, h = fields["loop"].split("x")
            loop = (float(w), float(h))
        geom = None
        if fields.get("geom", "-") != "-":
            x0, y0, z0, x1, y1, z1 = map(float, fields["geom"].split(","))
            geom = (x1 - x0, y1 - y0, z1 - z0)
        rows[parts[1]] = {
            "class": parts[2],
            "symbol": parts[3],
            "loops": fields.get("loops"),
            "complex": fields.get("complex"),
            "loop": loop,
            "geom": geom,
            "faces": fields.get("faces"),
            "classes": fields.get("classes"),
        }
    return rows


def close(a, b):
    return a is not None and b is not None and abs(a - b) <= TOLERANCE_FEET


def main():
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    rows = read_probe(args[0])
    revit = revit_sizes(args[1])
    print(f"probe: {len(rows)} doors and windows; Revit's export: {len(revit)}")
    kinds = collections.Counter()
    table = collections.defaultdict(collections.Counter)
    listing = []
    for tag, row in sorted(rows.items(), key=lambda kv: (len(kv[0]), kv[0])):
        if tag not in revit:
            kinds["not in Revit's export"] += 1
            continue
        _, width, height = revit[tag]
        if width is None or height is None:
            kinds["Revit writes no size"] += 1
            continue
        kind = ("loop" if row["loop"] else "no loop") + (", cut geometry" if row["geom"] else ", no cut geometry")
        kinds[kind] += 1
        cells = table[(row["class"], kind)]
        cells["n"] += 1
        if row["loop"]:
            cells["loop = Revit"] += close(row["loop"][0], width) and close(row["loop"][1], height)
        if row["geom"]:
            gx, gy, gz = row["geom"]
            cells["geom X,Z = Revit"] += close(gx, width) and close(gz, height)
            cells["geom X,Y = Revit"] += close(gx, width) and close(gy, height)
            cells["geom X = width"] += close(gx, width)
        listing.append((tag, row, width, height))
    for kind, n in kinds.most_common():
        print(f"  {kind}: {n}")
    print("class / kind: counts")
    for (cls, kind), cells in sorted(table.items()):
        print(f"  {cls:<9} {kind:<30} " + ", ".join(f"{k} {v}" for k, v in cells.items()))
    print("no loop, by symbol (Revit width x height vs cut geometry X x Y x Z)")
    by_symbol = collections.defaultdict(list)
    for tag, row, width, height in listing:
        if not row["loop"]:
            by_symbol[row["symbol"]].append((tag, row, width, height))
    for symbol, items in sorted(by_symbol.items(), key=lambda kv: -len(kv[1]))[:30]:
        tag, row, width, height = items[0]
        geom = "x".join(f"{v:.4f}" for v in row["geom"]) if row["geom"] else "-"
        print(
            f"  symbol {symbol}: {len(items)} {row['class']} (e.g. {tag}) Revit {width:.4f}x{height:.4f}"
            f" geom {geom} complex={row['complex']} faces={row['faces']} {row['classes']}"
        )
    if "--list" in sys.argv:
        for tag, row, width, height in listing:
            geom = "x".join(f"{v:.4f}" for v in row["geom"]) if row["geom"] else "-"
            loop = "x".join(f"{v:.4f}" for v in row["loop"]) if row["loop"] else "-"
            print(f"row {tag} {row['class']} {row['symbol']} Revit {width:.4f}x{height:.4f} loop {loop} geom {geom}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

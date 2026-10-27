#!/usr/bin/env python3
"""Whether a wall's `Height` is Revit's where its two readings agree
(#1016, RE-257).

revit-ifc's `WallExporter` (CLEANROOM.md source 5) writes a wall's
`Qto_WallBaseQuantities.Height` as the vertical span of the wall's solids.
rvt-rs has two readings of it: the depth of the extrusion it draws the wall
as (`B.depth`, from its record box) and the wall's saved Unconnected Height
(`P.-1001101`, RE-251). Each is Revit's on most walls but not all. This
reads `examples/probe_re251_wall_numbers.rs`'s `rec` lines and Revit's IFC
export, and counts, for the walls where the two agree (a relative 1e-6),
how many carry Revit's `Height`, a different one, or none; then, for the
walls where they do not, which of the two Revit's is. It also counts the
walls Revit's export writes more than once (a wall split by storey).

Usage:

    python3 tools/re/wall_height_vs_qtos.py <probe.txt> <revit.ifc> [--list]
"""

import collections
import os
import sys

import ifcopenshell

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from qtos_vs_ifc import quantities  # noqa: E402
from wall_numbers_vs_qtos import FOOT, read_probe  # noqa: E402

DEPTH = "B.depth"
UNCONNECTED = "P.-1001101"


def same(a, b):
    return abs(a - b) <= 1e-6 * max(1.0, abs(a), abs(b))


def main():
    listing = "--list" in sys.argv
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    rows = read_probe(args[0])
    heights = {}
    for (key, qset, name), (_, value) in quantities(args[1])[0].items():
        if qset == "Qto_WallBaseQuantities" and name == "Height" and value is not None:
            heights[key.removeprefix("tag:")] = value / FOOT
    written = collections.Counter(
        str(wall.Tag) for wall in ifcopenshell.open(args[1]).by_type("IfcWall") if wall.Tag
    )
    counts = collections.Counter()
    lines = []
    for tag, fields in sorted(rows.items()):
        depth = fields.get(DEPTH, [None])[0]
        unconnected = fields.get(UNCONNECTED, [None])[0]
        revit = heights.get(tag)
        if written.get(tag, 0) > 1:
            counts["written more than once"] += 1
        if depth is None or unconnected is None:
            group = "no depth" if depth is None else "no unconnected height"
        elif same(depth, unconnected):
            group = "agree"
        else:
            group = "disagree"
        if revit is None:
            outcome = "Revit writes none"
        elif group == "agree":
            outcome = "Revit's" if same(depth, revit) else "different"
        else:
            hits = [n for n, v in ((DEPTH, depth), (UNCONNECTED, unconnected)) if v is not None and same(v, revit)]
            outcome = " and ".join(hits) + " is Revit's" if hits else "neither is Revit's"
        counts[(group, outcome)] += 1
        if group != "agree" or outcome != "Revit's":
            lines.append(f"  tag:{tag} {group}: depth {depth}, unconnected {unconnected}, Revit {revit}")
    print(f"probe: {len(rows)} walls, Revit's export: {len(heights)} with a Height")
    for key, n in sorted(counts.items(), key=lambda kv: str(kv[0])):
        label = key if isinstance(key, str) else f"{key[0]}: {key[1]}"
        print(f"  {label}: {n}")
    if listing:
        print("\n".join(lines))
    return 0


if __name__ == "__main__":
    sys.exit(main())

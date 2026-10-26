#!/usr/bin/env python3
"""How a wall's Length in Revit's export differs from its centreline, by the
walls it is joined to (#1016, #1046).

revit-ifc's `LengthCalculator` (CLEANROOM.md source 5) takes a wall's
`Qto_WallBaseQuantities.Length` from its Length parameter, which on the
BIM4LCA office is not its centreline's length on 186 of 607 walls. Reads
`examples/probe_re70_wall_join_lists.rs`'s JSON lines (each wall's centreline,
type thickness and join lists) and Revit's IFC export, and for each wall
takes the centreline's length less Revit's `Length` (feet). It looks for that
difference among the sums of one term per join list, each term none, or plus
or minus half or the whole of the thickness of a wall the list names (within
1e-4 ft), and tallies the walls by the terms that give it: `0` for none,
`+h`/`-h` for half, `+t`/`-t` for whole, per list, and `?` where nothing does.
Up to 30 walls with no match are listed.

Usage:

    python3 tools/re/wall_length_vs_joins.py <probe.jsonl> <revit.ifc>
"""

import collections
import itertools
import json
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from qtos_vs_ifc import quantities  # noqa: E402

FOOT = 0.3048
TOL = 1e-4


def main(argv):
    args = [a for a in argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    walls = {}
    for line in open(args[0], encoding="utf-8"):
        line = line.strip()
        if line.startswith("{"):
            row = json.loads(line)
            walls[row["id"]] = row
    thickness = {i: w["thickness"] for i, w in walls.items()}
    revit = {}
    for (key, qset, name), (_, value) in quantities(args[1])[0].items():
        if qset == "Qto_WallBaseQuantities" and name == "Length" and value is not None:
            tag = key.removeprefix("tag:")
            if tag.isdigit():
                revit[int(tag)] = value / FOOT
    tally = collections.Counter()
    unmatched = []
    for i, length in sorted(revit.items()):
        w = walls.get(i)
        if w is None:
            tally["no probe row"] += 1
            continue
        centre = math.dist(w["start"], w["end"])
        diff = centre - length
        lists = (w.get("lists") or [[]])[0][:2]
        while len(lists) < 2:
            lists.append([])
        options = []
        for entries in lists:
            terms = [("0", 0.0)]
            for _, other, _ in entries:
                t = thickness.get(other)
                if t is None:
                    continue
                terms += [("+h", t / 2), ("-h", -t / 2), ("+t", t), ("-t", -t)]
            options.append(terms)
        found = None
        for combo in itertools.product(*options):
            if abs(sum(v for _, v in combo) - diff) <= TOL:
                found = ",".join(label for label, _ in combo)
                break
        if found is None:
            tally["?"] += 1
            unmatched.append((i, diff, [len(e) for e in lists], w["thickness"]))
        else:
            tally[found] += 1
    print(f"walls with Revit's Length: {len(revit)}; with a probe row: {sum(1 for i in revit if i in walls)}")
    print("centreline less Revit's Length, by the terms (one per join list) that give it:")
    for label, n in tally.most_common():
        print(f"  {label}: {n}")
    print("no match:")
    for i, diff, sizes, t in unmatched[:30]:
        print(f"  {i}: {diff:+.6f} ft, lists of {sizes}, own thickness {t:.6g}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

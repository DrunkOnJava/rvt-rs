#!/usr/bin/env python3
"""How a wall's Length in Revit's export follows from the joins at its ends
(#1046, RE-252).

Reads `examples/probe_re252_wall_ends.rs`'s JSON lines and Revit's IFC
export. For each wall it takes the centreline's length less Revit's `Length`
(feet) and, per end of the wall, looks for that difference among the sums of one term per end: none,
or plus or minus half or the whole of the thickness of a wall joined there,
or half the wall's own (within 1e-4 ft). A wall's `CDJoinInfo` records the
walls whose end meets one of its sides (its own entry 2 or 3, theirs 0 or 1),
so the walls joined at a wall's end are those whose records name that end.
It tallies the walls by the number of walls joined at each end and whether a
sum gives the difference, by the terms that do, and lists up to 40 walls
nothing gives.

Usage:

    python3 tools/re/wall_length_vs_ends.py <probe.jsonl> <revit.ifc>
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


def joined_ends(walls):
    """(wall id, end 0 or 1) -> [(id of the wall it meets, that wall's side)]."""
    out = collections.defaultdict(list)
    for host in walls.values():
        for join in host.get("joins", []):
            pairs = [p for p in join[:-1] if isinstance(p, list)]
            side = [e for i, e in pairs if i == host["id"]]
            for other, end in pairs:
                if other != host["id"] and end in (0, 1):
                    out[(other, end)].append((host["id"], side[0] if side else None))
    return out


def kind(joined):
    return str(len(joined)) if joined else "-"


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
    revit = {}
    for (key, qset, name), (_, value) in quantities(args[1])[0].items():
        if qset == "Qto_WallBaseQuantities" and name == "Length" and value is not None:
            tag = key.removeprefix("tag:")
            if tag.isdigit():
                revit[int(tag)] = value / FOOT
    meets = joined_ends(walls)
    by_kind = collections.Counter()
    by_terms = collections.Counter()
    unmatched = []
    for i, length in sorted(revit.items()):
        w = walls.get(i)
        if w is None:
            continue
        diff = math.dist(w["start"], w["end"]) - length
        joined = {end: meets.get((i, end), []) for end in (0, 1)}
        own = w["thickness"]
        options = []
        for end in (0, 1):
            terms = [("0", 0.0), ("+own/2", own / 2), ("-own/2", -own / 2)]
            for other, _ in joined[end]:
                t = walls.get(other, {}).get("thickness")
                if t is None:
                    continue
                terms += [("+h", t / 2), ("-h", -t / 2), ("+t", t), ("-t", -t)]
            options.append(terms)
        found = None
        for combo in itertools.product(*options):
            if abs(sum(v for _, v in combo) - diff) <= TOL:
                found = ",".join(label for label, _ in combo)
                break
        k = f"{kind(joined[0])}{kind(joined[1])}"
        by_kind[(k, found is not None)] += 1
        if found is None:
            unmatched.append((i, diff, k, own, joined))
        else:
            by_terms[(k, found)] += 1
    print(f"walls with Revit's Length and a probe row: {sum(by_kind.values())}")
    print("joins at the two ends, a sum gives the difference: walls")
    for (k, ok), n in sorted(by_kind.items(), key=lambda kv: -kv[1]):
        print(f"  {k} {ok}: {n}")
    print("joins at the two ends, the terms that give it: walls")
    for (k, terms), n in sorted(by_terms.items(), key=lambda kv: -kv[1])[:30]:
        print(f"  {k} {terms}: {n}")
    print("nothing gives the difference:")
    for i, diff, k, own, joined in unmatched[:40]:
        print(f"  {i}: {diff:+.6f} ft, joins {k}, own {own:.6g}, at 0 {joined[0]}, at 1 {joined[1]}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

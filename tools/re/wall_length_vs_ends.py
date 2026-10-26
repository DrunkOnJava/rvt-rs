#!/usr/bin/env python3
"""How a wall's Length in Revit's export follows from the walls joined at its
ends (#1046, RE-252).

Reads `examples/probe_re252_wall_ends.rs`'s JSON lines and Revit's IFC
export. A wall's `VWallDriver.m_controlJoinsSet` names, per end
(`m_index`), the walls whose location lines meet there. For each wall it
takes, per end, either the centreline's end or the point where its location
line crosses a wall the end's set names (where the two are parallel, that
wall's named end, projected), and compares the distance between the two
points along the wall with Revit's `Length` (feet, within 1e-4 ft). A
location line is the centreline moved `m_keyRefOffset` along the exterior
direction, scaled by -1, 0 or +1 for all walls (the sign is not known).
It prints, per sign, the walls a choice of points gives the Length on, by
the number of walls each end's set names and by the points chosen, and lists
up to 40 walls none does.

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


def line(wall, sign):
    """The wall's location line as (point, unit direction, length)."""
    (sx, sy), (ex, ey) = wall["start"], wall["end"]
    length = math.hypot(ex - sx, ey - sy)
    d = ((ex - sx) / length, (ey - sy) / length)
    offset = 0.0
    if sign and wall.get("keyref") and wall.get("exterior"):
        offset = sign * (wall["keyref"][2] or 0.0)
    nx, ny = wall["exterior"] if wall.get("exterior") else (0.0, 0.0)
    return (sx + offset * nx, sy + offset * ny), d, length


def meet(own, other, other_end):
    """Where `own` reaches `other`'s location line, as a distance along `own`."""
    (px, py), (dx, dy), _ = own
    (qx, qy), (ex, ey), olen = other
    cross = dx * ey - dy * ex
    if abs(cross) < 1e-9:
        tx, ty = (qx, qy) if other_end == 0 else (qx + olen * ex, qy + olen * ey)
        return (tx - px) * dx + (ty - py) * dy
    return ((qx - px) * ey - (qy - py) * ex) / cross


def options(walls, wall, sign):
    own = line(wall, sign)
    sets = {}
    for index, entries in wall.get("control", []):
        sets[index] = [(o, e) for o, e in entries if o != wall["id"] and o in walls]
    out = []
    for end in (0, 1):
        choices = [("axis", 0.0 if end == 0 else own[2])]
        for other, other_end in sets.get(end, []):
            choices.append((f"meet{other_end}", meet(own, line(walls[other], sign), other_end)))
        out.append(choices)
    return out, sets


def main(argv):
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    walls = {}
    for raw in open(argv[1], encoding="utf-8"):
        raw = raw.strip()
        if raw.startswith("{"):
            row = json.loads(raw)
            walls[row["id"]] = row
    revit = {}
    for (key, qset, name), (_, value) in quantities(argv[2])[0].items():
        if qset == "Qto_WallBaseQuantities" and name == "Length" and value is not None:
            tag = key.removeprefix("tag:")
            if tag.isdigit():
                revit[int(tag)] = value / FOOT
    for sign in (0, 1, -1):
        by_kind = collections.Counter()
        by_choice = collections.Counter()
        unmatched = []
        for i, length in sorted(revit.items()):
            wall = walls.get(i)
            if wall is None:
                continue
            ends, sets = options(walls, wall, sign)
            kind = "".join(str(len(sets.get(end, []))) for end in (0, 1))
            found = None
            for a, b in itertools.product(*ends):
                if abs(abs(b[1] - a[1]) - length) <= TOL:
                    found = f"{a[0]},{b[0]}"
                    break
            by_kind[(kind, found is not None)] += 1
            if found is None:
                best = min(abs(abs(b[1] - a[1]) - length) for a, b in itertools.product(*ends))
                unmatched.append((i, length, best, kind, wall.get("keyref"), sets))
            else:
                by_choice[(kind, found)] += 1
        total = sum(by_kind.values())
        hits = sum(n for (_, ok), n in by_kind.items() if ok)
        print(f"sign {sign:+d}: {hits} of {total} walls")
        for (k, ok), n in sorted(by_kind.items(), key=lambda kv: -kv[1]):
            print(f"  walls named at the ends {k}, given {ok}: {n}")
        for (k, c), n in sorted(by_choice.items(), key=lambda kv: -kv[1])[:20]:
            print(f"  {k} {c}: {n}")
        for i, length, best, k, keyref, sets in unmatched[:40]:
            print(f"  none: {i} Length {length:.6f} ft, nearest {best:.6f} off, named {k}, keyref {keyref}, sets {sets}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

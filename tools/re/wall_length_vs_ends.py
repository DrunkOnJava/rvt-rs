#!/usr/bin/env python3
"""How a wall's Length in Revit's export follows from the walls joined at its
ends (#1046, RE-252).

Reads `examples/probe_re252_wall_ends.rs`'s JSON lines and Revit's IFC
export. The walls joined at a wall's end are those its
`VWallDriver.m_controlJoinsSet` names for that end and those whose
`CDJoinInfo` pairs this wall's end with one of their sides. Each end of the
wall's `GLine` (its body's centreline) may stay where it is (`axis`) or stop
where the line crosses a plane of a wall joined there: its middle (`mid`),
the face nearer this wall's other end (`near`), the other face (`far`) or
another of its planes (`other`). For each wall it looks for a choice at each
end whose distance along the line is Revit's `Length` (feet, within 1e-4
ft), and prints how many walls one gives, the choices that do (the first
found, `axis` first), the walls by the choices at each end and whether
`axis, axis` alone gives it, and up to 40 walls none does. With `--list`,
it also prints a `wall <id> <Length> <choices>` line per wall, the choices
at each end as `[label, joined wall id, GLine parameter]`.

Usage:

    python3 tools/re/wall_length_vs_ends.py <probe.jsonl> <revit.ifc> [--list]
"""

import collections
import itertools
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from qtos_vs_ifc import quantities  # noqa: E402

FOOT = 0.3048
TOL = 1e-4


def gline(wall):
    ox, oy, dx, dy, e0, e1 = wall["gline"]
    return (ox, oy), (dx, dy), (e0, e1)


def offsets(wall):
    """Each plane's signed distance from the wall's GLine, and the plane."""
    (ox, oy), (dx, dy), _ = gline(wall)
    out = []
    for px, py, xx, xy in wall.get("planes", []):
        if abs(dx * xy - dy * xx) > 1e-6:
            continue
        out.append(((px - ox) * -dy + (py - oy) * dx, (px, py, xx, xy)))
    return out


def crossing(own, plane):
    """The GLine parameter of `own` where it crosses `plane`'s line, or None."""
    (ox, oy), (dx, dy), _ = own
    px, py, xx, xy = plane
    cross = dx * xy - dy * xx
    if abs(cross) < 1e-9:
        return None
    return ((px - ox) * xy - (py - oy) * xx) / cross


def joined(walls):
    """(wall id, end) -> ids of the walls joined there."""
    out = collections.defaultdict(set)
    for wall in walls.values():
        for index, entries in wall.get("control", []):
            for other, _ in entries:
                if other != wall["id"]:
                    out[(wall["id"], index)].add(other)
        for join in wall.get("joins", []):
            pairs = [p for p in join[:-1] if isinstance(p, list)]
            for other, end in pairs:
                if other != wall["id"] and end in (0, 1):
                    out[(other, end)].add(wall["id"])
    return out


def choices(walls, meets, wall, end):
    own = gline(wall)
    here, there = own[2][end], own[2][1 - end]
    out = [("axis", None, here)]
    for other in sorted(meets.get((wall["id"], end), ())):
        o = walls.get(other)
        if o is None or o.get("gline") is None:
            continue
        planes = offsets(o)
        if not planes:
            continue
        half = max(abs(off) for off, _ in planes)
        found = []
        for off, plane in planes:
            t = crossing(own, plane)
            if t is not None:
                found.append((off, t))
        for off, t in found:
            if abs(off) < 1e-6:
                label = "mid"
            elif abs(abs(off) - half) < 1e-6:
                faces = [u for v, u in found if abs(abs(v) - half) < 1e-6]
                label = "near" if abs(t - there) == min(abs(u - there) for u in faces) else "far"
            else:
                label = "other"
            out.append((label, other, t))
    return out


def main(argv):
    listing = "--list" in argv
    argv = [a for a in argv if a != "--list"]
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
    meets = joined(walls)
    by_choice = collections.Counter()
    by_kind = collections.Counter()
    unmatched = []
    total = 0
    for i, length in sorted(revit.items()):
        wall = walls.get(i)
        if wall is None or wall.get("gline") is None:
            continue
        total += 1
        ends = [choices(walls, meets, wall, end) for end in (0, 1)]
        found = None
        if listing:
            print(f"wall {i} {length!r} {json.dumps(ends)}")
        for a, b in itertools.product(*ends):
            if abs(abs(b[2] - a[2]) - length) <= TOL:
                found = (a[0], b[0])
                break
        kind = tuple(len(meets.get((i, end), ())) for end in (0, 1))
        by_kind[(kind, found is not None, found == ("axis", "axis"))] += 1
        if found is None:
            span = abs(ends[1][0][2] - ends[0][0][2])
            unmatched.append((i, length, span - length, kind, wall.get("keyref")))
        else:
            by_choice[tuple(sorted(found))] += 1
    print(f"walls with Revit's Length and a GLine: {total}")
    print(f"a choice at each end gives the Length: {sum(by_choice.values())}")
    for c, n in by_choice.most_common():
        print(f"  {c[0]}, {c[1]}: {n}")
    print("walls joined at the two ends, given, by axis alone: walls")
    for (k, ok, axis), n in sorted(by_kind.items(), key=lambda kv: -kv[1]):
        print(f"  {k[0]} {k[1]} {ok} {axis}: {n}")
    print("nothing gives the Length:")
    for i, length, diff, k, keyref in unmatched[:40]:
        print(f"  {i}: Length {length:.6f} ft, GLine less Length {diff:+.6f} ft, joined {k}, keyref {keyref}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

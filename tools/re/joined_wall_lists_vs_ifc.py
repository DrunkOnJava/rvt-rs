#!/usr/bin/env python3
"""Tabulate the joined-wall lists (RE-127 probe) against how Revit's body
ends at every L joint.

Input: the JSON lines of `examples/probe_re127_joined_wall_lists.rs` (other
lines are skipped) and Revit's export of the same model.

At each L joint (a wall end where exactly one other wall ends its
centreline at the same point, as `wall_join_lists_vs_ifc.py` finds them),
Revit's body is labelled by how far it reaches past the joint along the
wall: half the partner's thickness (`through`), minus half (`stopped`) or
neither (`other`). Each end is then counted by that label, by RE-70's rule
from the `join` lists, and by what the `joined` lists hold: whether the
wall's lists name the partner, whether the partner's name the wall, and
the tags involved. `--list` prints every end RE-70 leaves undecided or
gets wrong, with both walls' entries for each other.

Usage:

    python3 tools/re/joined_wall_lists_vs_ifc.py probe.txt revit-export.ifc [--list] [--ids ID,ID]

Needs IfcOpenShell (tested with 0.8.5).
"""

import collections
import importlib.util
import json
import math
import os
import sys

EPS = 1e-6


def helper(name):
    here = os.path.dirname(os.path.abspath(__file__))
    spec = importlib.util.spec_from_file_location(name, os.path.join(here, name + ".py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load(path):
    rows = {}
    for line in open(path):
        if line.startswith("{"):
            row = json.loads(line)
            rows[row["id"]] = row
    return rows


def overlap(a, b):
    """Whether two walls' record boxes overlap in height (NaN, unknown: yes)."""
    za, zb = a.get("z") or [None, None], b.get("z") or [None, None]
    if None in za + zb or any(math.isnan(v) for v in za + zb):
        return True
    return min(za[1], zb[1]) - max(za[0], zb[0]) > 1e-3


def l_joints(rows):
    ends = collections.defaultdict(list)
    for tag, row in rows.items():
        for which in ("start", "end"):
            ends[(round(row[which][0], 5), round(row[which][1], 5))].append(tag)
    for tag, row in rows.items():
        if math.hypot(row["end"][0] - row["start"][0], row["end"][1] - row["start"][1]) < 1e-9:
            continue
        for which, point in (("start", row["start"]), ("end", row["end"])):
            mates = [
                o
                for o in ends[(round(point[0], 5), round(point[1], 5))]
                if o != tag and overlap(rows[tag], rows[o])
            ]
            if len(mates) == 1:
                yield tag, which, point, mates[0]


def main():
    args = sys.argv[1:]
    if len(args) < 2:
        raise SystemExit(__doc__)
    listing = "--list" in args
    ids = {int(v) for v in args[args.index("--ids") + 1].split(",")} if "--ids" in args else set()
    rows = load(args[0])
    layers = helper("wall_layers_vs_ifc")
    import ifcopenshell
    import ifcopenshell.geom

    model = ifcopenshell.open(args[1])
    to_internal, _ = layers.site_to_internal(model)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    bodies = {}
    for wall in model.by_type("IfcWall"):
        try:
            tag = int(wall.Tag)
        except (TypeError, ValueError):
            continue
        if tag not in rows:
            continue
        try:
            shape = ifcopenshell.geom.create_shape(settings, wall)
        except Exception:
            continue
        verts = shape.geometry.verts
        bodies[tag] = [to_internal(verts[i], verts[i + 1]) for i in range(0, len(verts), 3)]

    def entries(wall, kind):
        """partner -> [(tag, word, payload)] from the wall's lists of `kind`."""
        out = collections.defaultdict(list)
        for frame in rows[wall]["lists"]:
            if frame["kind"] != kind:
                continue
            for entry in frame["entries"]:
                partner = entry[1] if kind == "join" else entry[0]
                out[partner].append((frame["tag"], frame["word"], entry))
        return out

    join = {wall: entries(wall, "join") for wall in rows}
    joined = {wall: entries(wall, "joined") for wall in rows}
    counts = collections.Counter()
    listed = []
    for wall, which, point, partner in l_joints(rows):
        if wall not in bodies:
            counts["no Revit body"] += 1
            continue
        row = rows[wall]
        length = math.hypot(row["end"][0] - row["start"][0], row["end"][1] - row["start"][1])
        d = ((row["end"][0] - row["start"][0]) / length, (row["end"][1] - row["start"][1]) / length)
        inward = d if which == "start" else (-d[0], -d[1])
        reach = -min((p[0] - point[0]) * inward[0] + (p[1] - point[1]) * inward[1] for p in bodies[wall])
        half = rows[partner]["thickness"] / 2
        role = "through" if abs(reach - half) < 1e-3 else "stopped" if abs(reach + half) < 1e-3 else "other"
        ours, theirs = partner in join[wall], wall in join[partner]
        rule = "through" if ours and not theirs else "stopped" if theirs and not ours else "undecided"
        a, b = joined[wall].get(partner, []), joined[partner].get(wall, [])
        key = (
            role,
            "RE-70 " + rule,
            "wall names partner" if a else "wall does not name partner",
            "partner names wall" if b else "partner does not name wall",
        )
        counts[key] += 1
        if (listing and (rule == "undecided" or rule != role)) or wall in ids:
            listed.append((wall, which, partner, role, round(reach, 4), a, b))
    for key, n in sorted(counts.items(), key=lambda kv: (-kv[1], str(kv[0]))):
        print(f"  {n:>5}  {' | '.join(key) if isinstance(key, tuple) else key}")
    for wall, which, partner, role, reach, a, b in listed:
        print(f"{wall} {which} x {partner}: Revit {role} ({reach} ft)")
        print(f"    wall's entries for partner: {a}")
        print(f"    partner's entries for wall: {b}")


if __name__ == "__main__":
    main()

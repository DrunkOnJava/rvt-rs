#!/usr/bin/env python3
"""Where Revit ends each wall's body, relative to the wall's native curve,
against where rvt-rs ends it and where a join-order rule would (W6).

Reads `examples/probe_wall_curves.rs` output (each wall's curve from its
native record, its location-line key, its end joins) and Revit's own IFC
export, and, when the Measure run keeps it (`-f keep_ifc=true`), rvt-rs's
export `model.ifc` beside the probe output. For every straight wall, each
body is meshed in internal feet and measured along the curve from its
start: how far it runs past the curve's start and past its end (negative:
it stops short).

The rule under test reads each end's join from the native record
(`VWallDriver.m_controlJoinsSet`, the walls in the join in the order the
record lists them):
- no other wall: the body ends at the curve's end;
- another wall meets it with that wall's middle (`m_joinedEnd` 2 or 3, a
  T): it stops half that wall's thickness short;
- otherwise (walls meeting end to end): the wall listed first runs half
  the second-listed wall's thickness past; every other wall stops half the
  first-listed wall's thickness short.

Per kind of end, how many ends Revit's body, rvt-rs's and the rule agree on
within 0.001 ft; then walls with both ends right for rvt-rs, for the rule,
and for the rule used only on the kinds where it beats rvt-rs.

Usage:

    python3 tools/re/wall_curve_ends_vs_ifc.py <probe output> <revit.ifc> [--list]
"""

import collections
import json
import math
import os
import re
import sys

import ifcopenshell
import ifcopenshell.geom

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wall_layers_vs_ifc import site_to_internal  # noqa: E402

TOLERANCE_FT = 0.001
TYPES = {}
CURVE = re.compile(
    r"^curve (\d+) GLine start (\S+) (\S+) end (\S+) (\S+) z (\S+) thickness (\S+) type (\S+) keyref (\S+) keyoffset (\S+) flipped (\S+)"
)


def parse(path):
    walls = {}
    for line in open(path, encoding="utf-8", errors="replace"):
        m = CURVE.match(line)
        if m:
            g = m.groups()
            walls[g[0]] = {
                "start": (float(g[1]), float(g[2])),
                "end": (float(g[3]), float(g[4])),
                "t": None if g[6] == "-" else float(g[6]),
                "type": g[7],
                "keyref": g[8],
                "flipped": g[10] == "true",
                "control": [],
            }
        elif line.startswith("walltype "):
            parts = line.split()
            layers = [float(w) for w in parts[3].split(",") if w]
            TYPES[parts[1]] = (layers, int(parts[5]) if parts[5].isdigit() else 0, int(parts[7]) if parts[7].isdigit() else 0)
        elif line.startswith("driver "):
            _, tag, _, _, _, control = line.rstrip("\n").split(" ", 5)
            if tag in walls:
                walls[tag]["control"] = json.loads(control) or []
    return walls


def past_ends(path, walls):
    """Tag -> (past start, past end) of the wall's body along its curve."""
    f = ifcopenshell.open(path)
    to_internal, _ = site_to_internal(f)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    out = {}
    for wall in f.by_type("IfcWall"):
        tag = str(wall.Tag) if wall.Tag else None
        if tag not in walls or not wall.Representation:
            continue
        body = next((r for r in wall.Representation.Representations if r.RepresentationIdentifier == "Body"), None)
        try:
            shape = ifcopenshell.geom.create_shape(settings, wall, body)
        except Exception:
            continue
        verts = shape.geometry.verts
        points = [to_internal(verts[i], verts[i + 1]) for i in range(0, len(verts), 3)]
        w = walls[tag]
        (sx, sy), (ex, ey) = w["start"], w["end"]
        length = math.hypot(ex - sx, ey - sy)
        if not points or length < 1e-6:
            continue
        ux, uy = (ex - sx) / length, (ey - sy) / length
        along = [(p[0] - sx) * ux + (p[1] - sy) * uy for p in points]
        out[tag] = (-min(along), max(along) - length)
    return out


def rule(tag, end, walls):
    """The reach the join-order rule gives `tag`'s `end`, and the kind of end."""
    w = walls[tag]
    joins = [
        j
        for j in w["control"]
        if any(str(i["m_elemId"]) == tag and i["m_joinedEnd"] == end for i in j["m_JoinInfo"]["m_info"])
    ]
    if not joins:
        return 0.0, "free"
    if len(joins) > 1:
        return None, "several joins"
    members = [i for i in joins[0]["m_JoinInfo"]["m_info"] if str(i["m_elemId"]) in walls]
    order = [str(i["m_elemId"]) for i in members]
    others = [i for i in members if str(i["m_elemId"]) != tag]
    if not others:
        return 0.0, "free"
    thickness = [walls[o]["t"] for o in order]
    if any(t is None for t in thickness):
        return None, "unknown thickness"
    tees = [i for i in others if i["m_joinedEnd"] in (2, 3)]
    if tees:
        if len(tees) > 1:
            return None, "several tees"
        return -walls[str(tees[0]["m_elemId"])]["t"] / 2, "tee"
    size = "2" if len(order) == 2 else "3+"
    if order.index(tag) == 0:
        return thickness[1] / 2, f"first of {size}"
    return -thickness[0] / 2, f"later of {size}"


def core_tee(tag, end, walls, reach, kind):
    """The tee rule, stopping at the host's core: the stem runs through the
    host's shell layers on its side (`m_numShellLayersExt` / `Int`)."""
    if kind != "tee" or reach is None:
        return reach
    w = walls[tag]
    join = [
        j
        for j in w["control"]
        if any(str(i["m_elemId"]) == tag and i["m_joinedEnd"] == end for i in j["m_JoinInfo"]["m_info"])
    ][0]
    host = next(str(i["m_elemId"]) for i in join["m_JoinInfo"]["m_info"] if str(i["m_elemId"]) != tag and i["m_joinedEnd"] in (2, 3))
    h = walls.get(host)
    shells = TYPES.get(h["type"]) if h else None
    if not shells:
        return None
    layers, ext, inner = shells
    (sx, sy), (ex, ey) = h["start"], h["end"]
    length = math.hypot(ex - sx, ey - sy)
    dx, dy = (ex - sx) / length, (ey - sy) / length
    exterior = (dy, -dx) if h["flipped"] else (-dy, dx)
    other = w["start"] if end == 1 else w["end"]
    side = (other[0] - sx) * exterior[0] + (other[1] - sy) * exterior[1]
    shell = sum(layers[:ext]) if side > 0 else sum(layers[len(layers) - inner:]) if inner else 0.0
    return reach + shell


def main():
    args = sys.argv[1:]
    if len(args) < 2:
        raise SystemExit(__doc__)
    walls = parse(args[0])
    revit = past_ends(args[1], walls)
    ours_path = os.path.join(os.path.dirname(os.path.abspath(args[0])), "model.ifc")
    ours = past_ends(ours_path, walls) if os.path.exists(ours_path) else {}
    print(f"walls with a native line: {len(walls)}; with Revit's body {len(revit)}; with rvt-rs's body {len(ours)}")
    table = collections.defaultdict(collections.Counter)
    per_wall = {}
    for tag, theirs in revit.items():
        w = walls[tag]
        for end in (0, 1):
            reach, kind = rule(tag, end, walls)
            reach = core_tee(tag, end, walls, reach, kind)
            key = f"{kind}, keyref {w['keyref']}"
            ok_rule = reach is not None and abs(reach - theirs[end]) <= TOLERANCE_FT
            ok_ours = tag in ours and abs(ours[tag][end] - theirs[end]) <= TOLERANCE_FT
            table[key]["n"] += 1
            table[key]["rvt-rs"] += ok_ours
            table[key]["rule"] += ok_rule
            table[key]["both"] += ok_ours and ok_rule
            per_wall.setdefault(tag, []).append((key, ok_ours, ok_rule, theirs[end], reach, ours.get(tag, (None, None))[end]))
    use_rule = {k for k, c in table.items() if c["rule"] > c["rvt-rs"]}
    print(f"{'kind of end':<34} {'ends':>5} {'rvt-rs':>7} {'rule':>6} {'both':>6}")
    for key, c in sorted(table.items(), key=lambda kv: -kv[1]["n"]):
        mark = "  <- rule" if key in use_rule else ""
        print(f"{key:<34} {c['n']:>5} {c['rvt-rs']:>7} {c['rule']:>6} {c['both']:>6}{mark}")
    right = collections.Counter()
    for tag, ends in per_wall.items():
        right["rvt-rs"] += all(e[1] for e in ends)
        right["rule"] += all(e[2] for e in ends)
        right["combined"] += all((e[2] if e[0] in use_rule else e[1]) for e in ends)
    print(f"walls with both ends within {TOLERANCE_FT} ft of Revit's: " + ", ".join(f"{k} {v}" for k, v in right.items()) + f" of {len(per_wall)}")
    if "--list" in args:
        print("ends the rule misses (tag, kind, Revit's past, rule's, rvt-rs's, type):")
        for tag, ends in sorted(per_wall.items()):
            for key, ok_ours, ok_rule, theirs, reach, mine in ends:
                if not ok_rule:
                    r = "-" if reach is None else f"{reach:+.4f}"
                    m = "-" if mine is None else f"{mine:+.4f}"
                    print(f"  {tag} {key}: Revit {theirs:+.4f} rule {r} rvt-rs {m} type {walls[tag]['type']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

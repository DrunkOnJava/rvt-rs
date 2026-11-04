#!/usr/bin/env python3
"""Where Revit ends each wall at a corner, by what the wall's native record
says about the join, against where rvt-rs ends it (RE-322, #1194).

Reads `probes/examples/probe_re322_wall_corners.rs` output (each wall's curve,
join lists and steps from its native record, each wall type's layers), Revit's
own IFC export and, when the Measure run keeps it (`-f keep_ifc=true`),
rvt-rs's export `model.ifc` beside the probe output. For every straight wall
in Revit's export, each body is meshed in internal feet and measured along the
wall's native curve: how far it runs past the curve's start and past its end,
on the left side and on the right side of the curve, and in all (the larger
of the two; negative where the body stops short).

Each end is classified from the record's join lists (`VWallDriver
.m_controlJoinsSet`):
- free: no wall is joined there;
- tee: another wall meets the end with its middle (`m_joinedEnd` 2 or 3);
- L2: exactly one other wall meets the end end to end, perpendicular,
  angled (45 to 135 degrees) or otherwise (collinear or a shallow bend);
- L3: two or more do.
Each is split by whether either wall has several layers.

Prints, per class of end, how many ends Revit's body and rvt-rs's agree on to
0.001 ft, the walls with both ends right, the most common differences, and,
for every end of a wall in both files, a `DATA` line with its measurements
and the record's steps at that end, for analysis away from the runner.

Usage:

    python3 tools/re/wall_corner_ends_vs_ifc.py <probe output> <revit.ifc> [--list]

Needs IfcOpenShell.
"""

import collections
import json
import math
import os
import sys

import ifcopenshell
import ifcopenshell.geom

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wall_layers_vs_ifc import site_to_internal  # noqa: E402

TOLERANCE_FT = 0.001
SIDE_FT = 0.001
FEET = 0.3048


def parse(path):
    walls, types = {}, {}
    for line in open(path, encoding="utf-8", errors="replace"):
        if not line.startswith("{"):
            continue
        try:
            row = json.loads(line)
        except ValueError:
            continue
        if "wall" in row:
            walls[str(row["wall"])] = row
        elif "type" in row:
            types[str(row["type"])] = row
    return walls, types


def objects(wall, name):
    return [o["f"] for o in wall["objects"] if o["c"] == name and isinstance(o["f"], dict)]


def controls(wall):
    drivers = objects(wall, "VWallDriver")
    if not drivers:
        return []
    return drivers[0].get("m_controlJoinsSet") or []


def wall_type(walls, types, tag):
    root = walls[tag]["root"]
    type_id = root.get("m_WallAttributesId") if isinstance(root, dict) else None
    return types.get(str(type_id)), str(type_id)


def thickness(walls, types, tag):
    t, _ = wall_type(walls, types, tag)
    if not t:
        return None
    return sum(l["w"] for l in t["layers"] if l.get("w"))


def layer_count(walls, types, tag):
    t, _ = wall_type(walls, types, tag)
    return len([l for l in t["layers"] if l.get("w")]) if t else None


def bodies(path, walls):
    """Tag -> measurements of the wall's body along its native curve."""
    f = ifcopenshell.open(path)
    to_internal, _ = site_to_internal(f)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    out = {}
    for wall in f.by_type("IfcWall"):
        tag = str(wall.Tag) if wall.Tag else None
        if tag not in walls or not wall.Representation or not walls[tag].get("curve"):
            continue
        body = next((r for r in wall.Representation.Representations if r.RepresentationIdentifier == "Body"), None)
        if body is None:
            continue
        try:
            shape = ifcopenshell.geom.create_shape(settings, wall, body)
        except Exception:
            continue
        verts = shape.geometry.verts
        points = [(*to_internal(verts[i], verts[i + 1]), verts[i + 2] / FEET) for i in range(0, len(verts), 3)]
        curve = walls[tag]["curve"]
        (sx, sy), (ex, ey) = curve["start"], curve["end"]
        length = math.hypot(ex - sx, ey - sy)
        if not points or length < 1e-6:
            continue
        ux, uy = (ex - sx) / length, (ey - sy) / length
        along = [(p[0] - sx) * ux + (p[1] - sy) * uy for p in points]
        across = [-(p[0] - sx) * uy + (p[1] - sy) * ux for p in points]
        sides = {}
        for name, keep in (("left", lambda a: a > SIDE_FT), ("right", lambda a: a < -SIDE_FT)):
            picked = [i for i, a in enumerate(across) if keep(a)]
            if picked:
                sides[name] = (-min(along[i] for i in picked), max(along[i] for i in picked) - length)
        kinds = sorted({item.is_a() for item in body.Items}) if body.Items else []
        out[tag] = {
            "past": (-min(along), max(along) - length),
            "sides": sides,
            "across": (min(across), max(across)),
            "z": (min(p[2] for p in points), max(p[2] for p in points)),
            "kinds": kinds,
        }
    return out


def end_join(wall, tag, end):
    found = []
    for control in controls(wall):
        members = (control.get("m_JoinInfo") or {}).get("m_info") or []
        if any(str(m["m_elemId"]) == tag and m["m_joinedEnd"] == end for m in members):
            found.append(control)
    return found


def classify(walls, types, tag, end):
    """(kind, detail, partners) of the join at `tag`'s `end`."""
    wall = walls[tag]
    found = end_join(wall, tag, end)
    if not found:
        return "free", "-", []
    if len(found) > 1:
        return "several joins", "-", []
    members = [m for m in found[0]["m_JoinInfo"]["m_info"] if str(m["m_elemId"]) != tag]
    if not members:
        return "free", "-", []
    if any(m["m_joinedEnd"] in (2, 3) for m in members):
        return "tee", "-", [(str(m["m_elemId"]), m["m_joinedEnd"]) for m in members]
    partners = [(str(m["m_elemId"]), m["m_joinedEnd"]) for m in members]
    layered = layer_count(walls, types, tag)
    if len(members) > 1:
        return "L3", f"{len(members)} walls", partners
    other, other_end = partners[0]
    other_wall = walls.get(other)
    if not other_wall or not other_wall.get("curve") or not wall.get("curve"):
        return "L2", "partner unread", partners
    cs, ce = wall["curve"]["start"], wall["curve"]["end"]
    os_, oe = other_wall["curve"]["start"], other_wall["curve"]["end"]
    here = cs if end == 0 else ce
    there = ce if end == 0 else cs
    far = oe if other_end == 0 else os_
    a = (there[0] - here[0], there[1] - here[1])
    b = (far[0] - here[0], far[1] - here[1])
    na, nb = math.hypot(*a), math.hypot(*b)
    if na < 1e-9 or nb < 1e-9:
        return "L2", "degenerate", partners
    cosine = (a[0] * b[0] + a[1] * b[1]) / (na * nb)
    if abs(cosine) <= 1e-6:
        angle = "perpendicular"
    elif cosine < -0.9999:
        angle = "collinear"
    elif abs(cosine) <= math.sqrt(0.5):
        angle = "angled"
    else:
        angle = "shallow"
    other_layers = layer_count(walls, types, other)
    layers = "single" if layered == 1 and other_layers == 1 else "layered" if layered and other_layers else "unknown"
    return "L2", f"{angle}, {layers}", partners


def state(error):
    if abs(error) <= TOLERANCE_FT:
        return "ok"
    return "short" if error < 0 else "long"


def main():
    args = sys.argv[1:]
    if len(args) < 2:
        raise SystemExit(__doc__)
    walls, types = parse(args[0])
    revit = bodies(args[1], walls)
    ours_path = os.path.join(os.path.dirname(os.path.abspath(args[0])), "model.ifc")
    ours = bodies(ours_path, walls) if os.path.exists(ours_path) else {}
    print(f"walls in the probe: {len(walls)}; wall types: {len(types)}; with Revit's body {len(revit)}; with rvt-rs's body {len(ours)}")
    table = collections.defaultdict(collections.Counter)
    deltas = collections.defaultdict(collections.Counter)
    per_wall = collections.defaultdict(list)
    rows = []
    for tag in sorted(revit, key=int):
        if tag not in ours:
            continue
        wall = walls[tag]
        for end in (0, 1):
            kind, detail, partners = classify(walls, types, tag, end)
            r, o = revit[tag]["past"][end], ours[tag]["past"][end]
            ok = abs(r - o) <= TOLERANCE_FT
            key = f"{kind}: {detail}"
            table[key]["ends"] += 1
            table[key]["ok"] += ok
            if not ok:
                deltas[key][f"{state(o - r)} by {abs(o - r):.4f}"] += 1
            per_wall[tag].append((key, ok))
            rows.append(
                {
                    "tag": tag,
                    "end": end,
                    "kind": kind,
                    "detail": detail,
                    "partners": partners,
                    "type": wall_type(walls, types, tag)[1],
                    "revit": r,
                    "ours": o,
                    "revit_sides": revit[tag]["sides"],
                    "ours_sides": ours[tag]["sides"],
                    "revit_bodies": revit[tag]["kinds"],
                    "ours_bodies": ours[tag]["kinds"],
                    "z": revit[tag]["z"],
                    "steps": [
                        {"c": o2["c"], "f": o2["f"]}
                        for o2 in wall["objects"]
                        if o2["c"].endswith("Step")
                        and isinstance(o2["f"], dict)
                        and (o2["c"] in ("WallJoinTweakGStep", "CleanPairGStep", "CutPairGStep") or o2["f"].get("m_end") == end)
                    ],
                    "nojoin": next(iter(objects(wall, "VWallDriver")), {}).get("m_noJoinAtEnd"),
                }
            )
    print(f"{'class of end':<44} {'ends':>5} {'ok':>5} {'off':>5}")
    for key, c in sorted(table.items(), key=lambda kv: -kv[1]["ends"]):
        print(f"{key:<44} {c['ends']:>5} {c['ok']:>5} {c['ends'] - c['ok']:>5}")
    right = sum(1 for ends in per_wall.values() if all(ok for _, ok in ends))
    print(f"walls with both ends within {TOLERANCE_FT} ft of Revit's: {right} of {len(per_wall)}")
    off = collections.defaultdict(list)
    for tag, ends in per_wall.items():
        wrong = [key for key, ok in ends if not ok]
        if wrong:
            off[" + ".join(sorted(wrong))].append(tag)
    print("walls not right, by the class of the end(s) that differ:")
    for key, tags in sorted(off.items(), key=lambda kv: -len(kv[1])):
        print(f"  {len(tags):>4}  {key}")
    print("most common differences (rvt-rs against Revit) per class of end that is off:")
    for key, counter in sorted(deltas.items(), key=lambda kv: -sum(kv[1].values())):
        top = ", ".join(f"{d} x{n}" for d, n in counter.most_common(6))
        print(f"  {key}: {top}")
    if "--list" in args:
        for row in rows:
            print("DATA " + json.dumps(row, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    sys.exit(main())

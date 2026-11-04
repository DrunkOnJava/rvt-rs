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
.m_controlJoinsSet`), counting only the walls that overlap the wall in
elevation (the list also names walls on other storeys that meet the same
point; elevation is that of Revit's bodies):
- free: no wall is joined there;
- tee: another wall meets the end with its middle (`m_joinedEnd` 2 or 3);
- L2: exactly one other wall meets the end end to end, perpendicular,
  angled (45 to 135 degrees) or otherwise (collinear or a shallow bend);
- L3, L4, ...: that many walls (this one included) meet end to end.
Each is split by whether any of the walls has several layers.

Prints, per class of end, how many ends Revit's body and rvt-rs's agree on to
0.001 ft, the walls with both ends right, the most common differences, the
order rule of `element_record_wall_joins::corner_joins` (RE-322) run over the
same data (how many ends it reads, how many it gets right and wrong, how many
ends it fixes or breaks, and why it leaves the rest), and, for every end of a
wall in both files, a `DATA` line with its measurements and the record's steps
at that end, for analysis away from the runner. The rule is read here from
Revit's bodies' elevations and the probe's data, so it leaves out a corner
with a wall that has no body in Revit's export, which the exporter does not.

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


def unit(a, b):
    d = (b[0] - a[0], b[1] - a[1])
    n = math.hypot(*d)
    return (d[0] / n, d[1] / n) if n > 1e-9 else None


def corner_members(walls, types, zs, tag, end):
    """The walls joined at `tag`'s `end` that overlap it in elevation, in the
    record's order, `tag` among them: dicts with the member's id, how it meets
    `tag`, its end point in `tag`'s frame (x out past the joint, y across),
    thickness and layer count. `[]` where no join lists the end, `None` where
    several do or a member cannot be placed."""
    found = end_join(walls[tag], tag, end)
    if not found:
        return []
    if len(found) > 1 or not walls[tag].get("curve") or tag not in zs:
        return None
    curve = walls[tag]["curve"]
    point = curve["start"] if end == 0 else curve["end"]
    far = curve["end"] if end == 0 else curve["start"]
    into = unit(point, far)
    if into is None:
        return None
    out, across = (-into[0], -into[1]), (-into[1], into[0])
    members = []
    for m in found[0]["m_JoinInfo"]["m_info"]:
        other, joined_end = str(m["m_elemId"]), m["m_joinedEnd"]
        if other == tag:
            members.append({"id": tag, "kind": "self", "t": thickness(walls, types, tag), "n": layer_count(walls, types, tag), "x": 0.0, "y": 0.0})
            continue
        if other not in zs or other not in walls or not walls[other].get("curve"):
            return None
        if min(zs[tag][1], zs[other][1]) - max(zs[tag][0], zs[other][0]) <= 0.01:
            continue
        t, n = thickness(walls, types, other), layer_count(walls, types, other)
        if joined_end in (2, 3):
            members.append({"id": other, "kind": "tee", "t": t, "n": n, "x": None, "y": None})
            continue
        oc = walls[other]["curve"]
        joint = oc["start"] if joined_end == 0 else oc["end"]
        beyond = oc["end"] if joined_end == 0 else oc["start"]
        away = unit(joint, beyond)
        if away is None:
            return None
        cosine = into[0] * away[0] + into[1] * away[1]
        kind = "perp" if abs(cosine) <= 1e-6 else "col" if cosine < -0.9999 else "angled" if abs(cosine) <= math.sqrt(0.5) else "shallow"
        offset = (joint[0] - point[0], joint[1] - point[1])
        members.append(
            {
                "id": other,
                "kind": kind,
                "t": t,
                "n": n,
                "x": offset[0] * out[0] + offset[1] * out[1],
                "y": offset[0] * across[0] + offset[1] * across[1],
            }
        )
    return members


def classify(members):
    """(kind, detail) of an end from its members (see `corner_members`)."""
    if members is None:
        return "unplaced", "a member has no straight line or body"
    others = [m for m in members if m["kind"] != "self"]
    if not others:
        return "free", "-"
    if any(m["kind"] == "tee" for m in others):
        return "tee", "-"
    layered = any((m["n"] or 0) > 1 for m in members)
    if len(others) == 1:
        angle = {"perp": "perpendicular", "col": "collinear", "angled": "angled", "shallow": "shallow"}[others[0]["kind"]]
        return "L2", f"{angle}, {'layered' if layered else 'single'}"
    return f"L{len(others) + 1}", "layered" if layered else "single"


LINE_FT = 0.001


def order_rule(walls, members, tag, end):
    """The order rule of `element_record_wall_joins::corner_joins` for an end
    where three or more walls meet end to end at a butt join: (reach in feet,
    which branch) or (None, why it is left alone)."""
    others = [m for m in members if m["kind"] != "self"]
    if len(others) < 2:
        return None, "fewer than three walls"
    controls = end_join(walls[tag], tag, end)
    if (controls[0]["m_JoinInfo"].get("m_wallJoinType") if controls else None) != 0:
        return None, "not a butt join"
    if any(m["kind"] not in ("perp", "col") for m in others):
        return None, "a wall meets it at another angle"
    if any(m["t"] is None or m["n"] is None or m["n"] < 1 for m in members):
        return None, "a wall's layers are not known"
    partners = set()
    for step in walls[tag]["objects"]:
        field = {"CleanPairGStep": "m_otherId", "CutPairGStep": "m_otherId", "WallEmbedWallGStep": "m_instId"}.get(step["c"])
        if field and isinstance(step["f"], dict):
            partners.add(str(step["f"].get(field)))
    if any(p not in walls for p in partners):
        return None, "an element other than a wall cuts it"
    me = next(i for i, m in enumerate(members) if m["kind"] == "self")
    cols = [(i, m) for i, m in enumerate(members) if m["kind"] == "col"]
    perps = [(i, m) for i, m in enumerate(members) if m["kind"] == "perp"]
    off_line = [m for _, m in cols if abs(m["x"]) > LINE_FT or abs(m["y"]) > LINE_FT]
    if any(m["n"] > 1 for m in off_line):
        return None, "a layered wall that continues it ends off the joint"
    if not perps:
        return None, "only walls that continue it"
    layered_wall = members[me]["n"] > 1
    layered_others = any(m["n"] > 1 for m in others)
    earlier = [m["x"] - m["t"] / 2 for i, m in perps if i < me]
    if earlier:
        if layered_wall and layered_others:
            return None, "a layered wall stops at layered walls"
        if max(earlier) - min(earlier) > LINE_FT:
            return None, "the earlier walls' faces differ"
        return min(earlier), "stops at an earlier wall"
    if layered_wall:
        return None, "a layered wall with no earlier wall to stop at"
    first = min(i for i, _ in perps if i > me)
    if any(i < first for i, _ in cols):
        if off_line:
            return None, "a wall that continues it ends off the joint"
        return 0.0, "meets the wall that continues it flush"
    far = [m["x"] + m["t"] / 2 for i, m in perps if i > me]
    if max(far) - min(far) > LINE_FT:
        return None, "the later walls' faces differ"
    return max(far), "runs through the later walls"


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
    rule = collections.defaultdict(collections.Counter)
    left_alone = collections.Counter()
    rows = []
    zs = {tag: body["z"] for tag, body in revit.items()}
    for tag in sorted(revit, key=int):
        if tag not in ours:
            continue
        wall = walls[tag]
        for end in (0, 1):
            members = corner_members(walls, types, zs, tag, end)
            kind, detail = classify(members)
            partners = [(m["id"], m["kind"]) for m in members or [] if m["kind"] != "self"]
            r, o = revit[tag]["past"][end], ours[tag]["past"][end]
            ok = abs(r - o) <= TOLERANCE_FT
            if members:
                reach, branch = order_rule(walls, members, tag, end)
            else:
                reach, branch = None, "no join" if members == [] else "a member has no straight line or body, or several joins list the end"
            if reach is None:
                left_alone[branch] += 1
            else:
                sides = [v[end] for v in revit[tag]["sides"].values()]
                right = abs(reach - r) <= TOLERANCE_FT and all(abs(reach - v) <= TOLERANCE_FT for v in sides)
                rule[branch]["fires"] += 1
                rule[branch]["right"] += right
                rule[branch]["wrong"] += not right
                rule[branch]["ends it fixes"] += right and not ok
                rule[branch]["ends it breaks"] += (not right) and ok
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
                    "rule": reach,
                    "rule_branch": branch,
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
    print("order rule (RE-322), ends where three or more walls meet end to end:")
    total = collections.Counter()
    for branch, c in sorted(rule.items()):
        print(f"  {branch}: " + ", ".join(f"{k} {v}" for k, v in c.items()))
        total.update(c)
    print("  all: " + (", ".join(f"{k} {v}" for k, v in total.items()) or "fires 0"))
    print("  left alone: " + ", ".join(f"{why} {n}" for why, n in left_alone.most_common()))
    if "--list" in args:
        for row in rows:
            print("DATA " + json.dumps(row, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    sys.exit(main())

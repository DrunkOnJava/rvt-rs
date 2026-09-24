#!/usr/bin/env python3
"""Score RE-70's wall join-list rule at every L joint of a model.

Research tool for `reports/element-framing/RE-70-wall-join-lists.md`. Its
input is the JSON-lines output of `examples/probe_re70_wall_join_lists.rs`:
each wall's centreline, type thickness, record box and join lists.

An L joint is a wall end where exactly one other wall, overlapping it in
elevation, ends its centreline at the same point (within 1e-6 ft). The rule
says the wall that names the other in its join lists, and is not named
back, runs on to the other's far face; the wall named stops at its near
face. Pairs that name each other or neither are undecided.

Two checks:

- **Record box** (always): at each perpendicular L joint of two
  axis-parallel walls, where the wall's own record box ends. The box reaches
  half the other wall's thickness past the joint (`through`), ends at the
  joint (`stopped`) or elsewhere (`other`).
- **Revit's body** (with a Revit IFC4 export): each end's two side faces in
  Revit's body, as ending on the other wall's far face, its near face or
  neither. Both on the far face is `through`, both on the near face
  `stopped`; one of each is `mitre-like` and anything else `irregular`.
  The perpendicular ends the rule decides are split by whether both walls
  have a single layer (the probe's `layers`).

Usage:

    python3 tools/re/wall_join_lists_vs_ifc.py walls.jsonl [revit-export.ifc]

Needs IfcOpenShell (tested with 0.8.5) for the IFC check only.
"""

import collections
import importlib.util
import json
import math
import os
import sys

EPS = 1e-6


def unit(v):
    length = math.hypot(*v)
    return (v[0] / length, v[1] / length)


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1]


def left(v):
    return (-v[1], v[0])


def load(path):
    rows = {row["id"]: row for row in map(json.loads, open(path))}
    names = {
        tag: {entry[1] for copy in row["lists"] for listed in copy for entry in listed}
        for tag, row in rows.items()
    }
    return rows, names


def l_joints(rows):
    """(wall, 's'|'e', joint point, partner) for every L joint end."""
    ends = collections.defaultdict(list)
    for tag, row in rows.items():
        for which in ("start", "end"):
            ends[(round(row[which][0], 5), round(row[which][1], 5))].append(tag)

    def overlap(a, b):
        box_a, box_b = rows[a].get("box"), rows[b].get("box")
        return not box_a or not box_b or min(box_a[5], box_b[5]) - max(box_a[2], box_b[2]) > 1e-3

    for tag, row in rows.items():
        start, end = row["start"], row["end"]
        if math.hypot(end[0] - start[0], end[1] - start[1]) < 1e-9:
            continue
        for which, point in (("s", start), ("e", end)):
            mates = [
                other
                for other in ends[(round(point[0], 5), round(point[1], 5))]
                if other != tag
                and overlap(tag, other)
                and any(
                    math.hypot(point[0] - rows[other][w][0], point[1] - rows[other][w][1]) <= EPS
                    for w in ("start", "end")
                )
            ]
            if len(mates) == 1:
                yield tag, which, point, mates[0]


def rule(names, wall, partner):
    ours, theirs = partner in names[wall], wall in names[partner]
    if ours and not theirs:
        return "through"
    if theirs and not ours:
        return "stopped"
    return "both" if ours else "neither"


def perpendicular(rows, wall, partner):
    a, b = rows[wall], rows[partner]
    d = unit((a["end"][0] - a["start"][0], a["end"][1] - a["start"][1]))
    m = unit((b["end"][0] - b["start"][0], b["end"][1] - b["start"][1]))
    return abs(dot(d, m)) <= EPS, d


def box_check(rows, names):
    counts = collections.Counter()
    for wall, which, point, partner in l_joints(rows):
        row = rows[wall]
        square, d = perpendicular(rows, wall, partner)
        if not square or "box" not in row or min(abs(d[0]), abs(d[1])) > 1e-9:
            continue
        inward = d if which == "s" else (-d[0], -d[1])
        box = row["box"]
        reach = max(
            -dot((x - point[0], y - point[1]), inward) for x in (box[0], box[3]) for y in (box[1], box[4])
        )
        half = rows[partner]["thickness"] / 2
        seen = "through" if abs(reach - half) < 1e-3 else "stopped" if abs(reach) < 1e-3 else "other"
        counts[(seen, rule(names, wall, partner))] += 1
    return counts


def ifc_check(rows, names, ifc_path):
    here = os.path.dirname(os.path.abspath(__file__))
    spec = importlib.util.spec_from_file_location("wall_layers_vs_ifc", os.path.join(here, "wall_layers_vs_ifc.py"))
    helpers = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helpers)
    import ifcopenshell
    import ifcopenshell.geom

    model = ifcopenshell.open(ifc_path)
    to_internal, _ = helpers.site_to_internal(model)
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
    counts = collections.Counter()
    split = collections.Counter()
    for wall, which, point, partner in l_joints(rows):
        if wall not in bodies:
            continue
        row, other = rows[wall], rows[partner]
        square, d = perpendicular(rows, wall, partner)
        inward = d if which == "s" else (-d[0], -d[1])
        across = left(inward)
        m = unit((other["end"][0] - other["start"][0], other["end"][1] - other["start"][1]))
        normal = left(m)
        faces = []
        for offset in (row["thickness"] / 2, -row["thickness"] / 2):
            face = [p for p in bodies[wall] if abs(dot((p[0] - point[0], p[1] - point[1]), across) - offset) < 1e-3]
            if not face:
                faces.append("none")
                continue
            reach = min(dot((p[0] - point[0], p[1] - point[1]), inward) for p in face)
            denominator = dot(inward, normal)
            label = "other"
            if abs(denominator) > 1e-6:
                hits = [
                    (side - offset * dot(across, normal)) / denominator
                    for side in (other["thickness"] / 2, -other["thickness"] / 2)
                ]
                if abs(reach - min(hits)) < 1e-3:
                    label = "far"
                elif abs(reach - max(hits)) < 1e-3:
                    label = "near"
            faces.append(label)
        if faces == ["far", "far"]:
            role = "through"
        elif faces == ["near", "near"]:
            role = "stopped"
        elif sorted(faces) == ["far", "near"]:
            role = "mitre-like"
        else:
            role = "irregular"
        predicted = rule(names, wall, partner)
        kind = "perpendicular" if square else "angled"
        counts[(kind, role, predicted)] += 1
        if role == "irregular":
            clean_face = {"far", "near"} & set(faces)
            if len(clean_face) == 1:
                counts[(kind, "irregular, clean face " + clean_face.pop(), predicted)] += 1
        decided = square and predicted in ("through", "stopped")
        if decided and "layers" in row and "layers" in other:
            single = row["layers"] == 1 and other["layers"] == 1
            clean = role in ("through", "stopped")
            split[("both single-layer" if single else "a multi-layer wall", "clean" if clean else "not clean")] += 1
    return counts, split


def main():
    args = sys.argv[1:]
    if not args:
        raise SystemExit(__doc__)
    rows, names = load(args[0])
    print(f"{len(rows)} walls with a centreline; {sum(1 for n in names.values() if n)} name a wall")
    print("record box against the rule (axis-parallel, perpendicular L joints):")
    for key, count in sorted(box_check(rows, names).items()):
        print(f"  {count:>5}  box {key[0]:<8} rule {key[1]}")
    if len(args) > 1:
        counts, split = ifc_check(rows, names, args[1])
        print("Revit's body against the rule (L joints):")
        for key, count in sorted(counts.items()):
            print(f"  {count:>5}  {key[0]:<13} {key[1]:<26} rule {key[2]}")
        for key, count in sorted(split.items()):
            print(f"  {count:>5}  {key[0]}: {key[1]}")


if __name__ == "__main__":
    main()

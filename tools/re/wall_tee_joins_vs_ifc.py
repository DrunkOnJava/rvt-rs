#!/usr/bin/env python3
"""Where a wall stops at a T joint, against Revit.

Research tool for `reports/element-framing/RE-73-wall-tee-joins.md`. Its
input is the JSON-lines output of `examples/probe_re73_tee_joins.rs`: each
wall's centreline, thickness, exterior side, layers (exterior first, each
width and function) and, at each end, the wall it joins and how far past
the centreline's end the exporter draws its body or each of its layers.

An end is a T joint where the wall it joins has the end on its centreline
away from both of its own ends. There Revit's IFC4 body is sliced in plan
along the middle of each of the wall's layers, and each layer's reach past
the end is compared with the exporter's; an end where a slice misses
Revit's body is counted as not measured.

Free ends (no join decided) that end on exactly one other wall's
centreline are also listed, by why the exporter leaves them alone (another
wall ends at the same point, the walls are not perpendicular, or the
other wall ends within its width) and by what Revit
draws there: every layer stopped at the other wall's face, at its
centreline, or otherwise.

Usage:

    python3 tools/re/wall_tee_joins_vs_ifc.py walls.jsonl revit-export.ifc

Needs IfcOpenShell (tested with 0.8.5) and shapely.
"""

import collections
import importlib.util
import json
import math
import os
import sys

import ifcopenshell
from shapely.geometry import LineString

HERE = os.path.dirname(os.path.abspath(__file__))
TOLERANCE = 2e-3
EPS = 1e-6
PERPENDICULAR = 1e-3


def load(name):
    spec = importlib.util.spec_from_file_location(name, os.path.join(HERE, name + ".py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


joins = load("wall_layer_joins_vs_ifc")


def unit(v):
    length = math.hypot(*v)
    return (v[0] / length, v[1] / length)


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1]


def on_line(row, point):
    """Where `point` lies along `row`'s centreline, and how far off it."""
    start, end = row["start"], row["end"]
    run = math.hypot(end[0] - start[0], end[1] - start[1])
    axis = ((end[0] - start[0]) / run, (end[1] - start[1]) / run)
    offset = (point[0] - start[0], point[1] - start[1])
    return dot(offset, axis), -offset[0] * axis[1] + offset[1] * axis[0], run, axis


def overlaps(a, b):
    box, other = a.get("box"), b.get("box")
    return bool(box and other) and min(box[5], other[5]) - max(box[2], other[2]) > EPS


def revit_reaches(row, body, point, into):
    """How far past `point` Revit draws each of the wall's layers, exterior
    first, or `None` for a layer the slice misses."""
    normal = unit(row["exterior"])
    edges = joins.boundaries(row)
    out = []
    for upper, lower in zip(edges, edges[1:]):
        mid = (upper + lower) / 2
        line = LineString([
            (point[0] + normal[0] * mid - into[0] * 50, point[1] + normal[1] * mid - into[1] * 50),
            (point[0] + normal[0] * mid + into[0] * 50, point[1] + normal[1] * mid + into[1] * 50),
        ])
        cut = joins.span(body.intersection(line), point, into)
        out.append(None if cut is None else -cut[0])
    return out


def shape(reaches, half):
    """What Revit draws at a T joint: every layer to the other wall's face,
    to its centreline, or otherwise."""
    if any(r is None for r in reaches):
        return "not measured"
    if all(abs(r + half) < TOLERANCE for r in reaches):
        return "every layer at the face"
    if all(abs(r) < TOLERANCE for r in reaches):
        return "every layer at the centreline"
    return "otherwise"


def main():
    args = sys.argv[1:]
    if len(args) < 2:
        raise SystemExit(__doc__)
    rows = {row["id"]: row for row in map(json.loads, open(args[0]))}
    helpers = joins.load("wall_layers_vs_ifc")
    model = ifcopenshell.open(args[1])
    to_internal, _ = helpers.site_to_internal(model)
    bodies = joins.footprints(model, rows, to_internal)
    decided = collections.Counter()
    by_layer = collections.Counter()
    declined = collections.Counter()
    for tag, row in rows.items():
        if tag not in bodies:
            continue
        start, end = row["start"], row["end"]
        along = unit((end[0] - start[0], end[1] - start[1]))
        for slot, point in ((0, start), (1, end)):
            into = along if slot == 0 else (-along[0], -along[1])
            join = row["ends"][slot]
            if join is not None:
                other = rows.get(join["wall"])
                if other is None:
                    continue
                at, across, run, _ = on_line(other, point)
                if not (EPS < at < run - EPS and abs(across) <= EPS):
                    continue
                # A layer's end is its two edges' reaches (RE-74); at its
                # middle line it is their mean.
                ours = [sum(pair) / 2 for pair in join["layer_reaches"]] if join["layer_reaches"] else [join["reach"]] * len(row["layers"])
                if any(r is None for r in ours):
                    continue
                theirs = revit_reaches(row, bodies[tag], point, into)
                kind = "single layer" if len(row["layers"]) == 1 else "layered"
                if any(r is None for r in theirs):
                    decided[(kind, "not measured")] += 1
                    continue
                right = True
                for mine, revit in zip(ours, theirs):
                    ok = abs(mine - revit) < TOLERANCE
                    by_layer["right" if ok else "wrong"] += 1
                    right &= ok
                decided[(kind, "every layer right" if right else "a layer wrong")] += 1
                if not right:
                    decided[(kind, "a layer wrong", shape(theirs, other["thickness"] / 2))] += 1
                continue
            hosts = []
            for other in rows.values():
                if other["id"] == tag or not overlaps(row, other):
                    continue
                at, across, run, axis = on_line(other, point)
                if EPS < at < run - EPS and abs(across) <= EPS:
                    hosts.append((other, at, run, axis))
            if len(hosts) != 1:
                continue
            other, at, run, axis = hosts[0]
            meets = [
                mate for mate in rows.values()
                if mate["id"] != tag and overlaps(row, mate)
                and any(math.hypot(point[0] - mate[w][0], point[1] - mate[w][1]) <= EPS for w in ("start", "end"))
            ]
            if meets:
                reason = "another wall also ends there"
            elif abs(dot(into, axis)) > PERPENDICULAR:
                reason = "not perpendicular"
            elif min(at, run - at) < row["thickness"] / 2 - EPS:
                reason = "the other wall ends within its width"
            else:
                reason = "unexplained"
            theirs = revit_reaches(row, bodies[tag], point, into)
            declined[(reason, shape(theirs, other["thickness"] / 2))] += 1
    print("T joints the exporter draws, against Revit's body:")
    for key, count in sorted(decided.items()):
        print(f"  {count:>5}  {', '.join(key)}")
    for key, count in sorted(by_layer.items()):
        print(f"  {count:>5}  layers {key}")
    print("free ends on one other wall's centreline the exporter leaves alone, by what Revit draws:")
    for key, count in sorted(declined.items()):
        print(f"  {count:>5}  {', '.join(key)}")


if __name__ == "__main__":
    main()

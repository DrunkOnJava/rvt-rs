#!/usr/bin/env python3
"""Where a wall ends at an angled join, against Revit.

Research tool for `reports/element-framing/RE-74-angled-wall-joins.md`. Its
input is the JSON-lines output of `examples/probe_re73_tee_joins.rs`: each
wall's centreline, thickness, exterior side, layers and, at each end, the
wall it joins and how far past the end each layer reaches along its
exterior-side and interior-side edge.

An end is an angled join where the wall it joins meets it at an angle other
than a right angle: an L joint where that wall's centreline ends at the same
point, a T joint where the end lies part way along it. For each such end the
exporter draws, Revit's IFC4 body is sliced in plan just inside each edge of
each layer, and the layer's reach there is compared with the exporter's.

Usage:

    python3 tools/re/wall_angled_joins_vs_ifc.py walls.jsonl revit-export.ifc

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
INSET = 2e-3
EPS = 1e-6


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


def revit_reach(body, point, into, normal, offset):
    """How far past `point` Revit's body reaches along the line `offset`
    from the centreline, or `None` where the slice misses it."""
    q = (point[0] + normal[0] * offset, point[1] + normal[1] * offset)
    line = LineString([
        (q[0] - into[0] * 60, q[1] - into[1] * 60),
        (q[0] + into[0] * 60, q[1] + into[1] * 60),
    ])
    cut = joins.span(body.intersection(line), q, into)
    return None if cut is None else -cut[0]


def main():
    args = sys.argv[1:]
    if len(args) < 2:
        raise SystemExit(__doc__)
    rows = {row["id"]: row for row in map(json.loads, open(args[0]))}
    helpers = joins.load("wall_layers_vs_ifc")
    model = ifcopenshell.open(args[1])
    to_internal, _ = helpers.site_to_internal(model)
    bodies = joins.footprints(model, rows, to_internal)
    counts = collections.Counter()
    by_layer = collections.Counter()
    for tag, row in rows.items():
        if tag not in bodies:
            continue
        start, end = row["start"], row["end"]
        along = unit((end[0] - start[0], end[1] - start[1]))
        normal = unit(row["exterior"])
        for slot, point in ((0, start), (1, end)):
            join = row["ends"][slot]
            other = join and rows.get(join["wall"])
            if not other:
                continue
            axis = unit((other["end"][0] - other["start"][0], other["end"][1] - other["start"][1]))
            cosine = abs(dot(along, axis))
            if cosine <= EPS or cosine >= 1 - EPS:
                continue
            at_end = any(math.hypot(point[0] - other[w][0], point[1] - other[w][1]) <= EPS for w in ("start", "end"))
            kind = ("L" if at_end else "T", "single layer" if len(row["layers"]) == 1 else "layered")
            pairs = join["layer_reaches"]
            if not pairs:
                counts[kind + ("no layer ends drawn",)] += 1
                continue
            into = along if slot == 0 else (-along[0], -along[1])
            edge = row["thickness"] / 2
            right = True
            measured = True
            for (width, _), pair in zip(row["layers"], pairs):
                for offset, ours in ((edge - INSET, pair[0]), (edge - width + INSET, pair[1])):
                    theirs = revit_reach(bodies[tag], point, into, normal, offset)
                    if theirs is None:
                        measured = False
                        continue
                    ok = abs(theirs - ours) < TOLERANCE
                    by_layer[(kind[0], "layer edge right" if ok else "layer edge wrong")] += 1
                    right &= ok
                edge -= width
            if not measured:
                counts[kind + ("not measured",)] += 1
            else:
                counts[kind + ("every layer edge right" if right else "a layer edge wrong",)] += 1
    print("angled joins the exporter draws, against Revit's body:")
    for key, count in sorted(counts.items()):
        print(f"  {count:>5}  {', '.join(key)}")
    for key, count in sorted(by_layer.items()):
        print(f"  {count:>5}  {', '.join(key)}")


if __name__ == "__main__":
    main()

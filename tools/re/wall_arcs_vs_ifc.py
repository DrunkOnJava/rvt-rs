#!/usr/bin/env python3
"""Curved walls' arcs, against the arc axes of Revit's own IFC export.

Research tool for `reports/element-framing/RE-75-curved-walls.md`. Its input
is the JSON-lines output of `examples/probe_re75_wall_arcs.rs`: each wall
whose data stores an arc, with the arc's centre, radius, angles and axes.

For every wall whose `Axis` representation in Revit's IFC4 export holds an
arc (an `IfcIndexedPolyCurve` with an `IfcArcIndex` segment), the arc's
centre and radius are found from its three points, turned into model feet,
and compared with the probe's. The ends are reported too, as the distance
between each end of the probe's arc and the nearer end of Revit's axis:
Revit's axis is the wall's trimmed at its joins.

Usage:

    python3 tools/re/wall_arcs_vs_ifc.py arcs.jsonl revit-export.ifc

Needs IfcOpenShell (tested with 0.8.5).
"""

import importlib.util
import json
import math
import os
import sys

import ifcopenshell
import ifcopenshell.util.placement

HERE = os.path.dirname(os.path.abspath(__file__))
TOLERANCE = 1e-3


def load(name):
    spec = importlib.util.spec_from_file_location(name, os.path.join(HERE, name + ".py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def circle(a, b, c):
    (ax, ay), (bx, by), (cx, cy) = a, b, c
    d = 2 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by))
    if abs(d) < 1e-12:
        return None
    ux = ((ax * ax + ay * ay) * (by - cy) + (bx * bx + by * by) * (cy - ay) + (cx * cx + cy * cy) * (ay - by)) / d
    uy = ((ax * ax + ay * ay) * (cx - bx) + (bx * bx + by * by) * (ax - cx) + (cx * cx + cy * cy) * (bx - ax)) / d
    return (ux, uy), math.hypot(ax - ux, ay - uy)


def revit_arcs(model, to_internal):
    """By `Tag`, the first arc of each wall's Axis: its centre, radius and the
    two ends of the axis curve, in model feet."""
    out = {}
    for wall in model.by_type("IfcWall"):
        try:
            tag = int(wall.Tag)
        except (TypeError, ValueError):
            continue
        for rep in (wall.Representation.Representations if wall.Representation else []):
            if rep.RepresentationIdentifier != "Axis":
                continue
            for item in rep.Items:
                if not (item.is_a("IfcIndexedPolyCurve") and item.Segments):
                    continue
                arcs = [s for s in item.Segments if s.is_a("IfcArcIndex")]
                if not arcs:
                    continue
                placement = ifcopenshell.util.placement.get_local_placement(wall.ObjectPlacement)
                points = []
                for x, y in (p[:2] for p in item.Points.CoordList):
                    world = placement @ [x, y, 0.0, 1.0]
                    points.append(to_internal(world[0], world[1]))
                found = circle(*[points[i - 1] for i in arcs[0].wrappedValue])
                if found:
                    out[tag] = (found[0], found[1], points[0], points[-1])
    return out


def main():
    args = sys.argv[1:]
    if len(args) < 2:
        raise SystemExit(__doc__)
    probe = {row["id"]: row for row in map(json.loads, open(args[0]))}
    model = ifcopenshell.open(args[1])
    to_internal, _ = load("wall_layers_vs_ifc").site_to_internal(model)
    revit = revit_arcs(model, to_internal)
    matched, ends = 0, []
    for tag, (centre, radius, first, last) in sorted(revit.items()):
        row = probe.get(tag)
        if row is None:
            print(f"  {tag}: Revit draws an arc axis; no arc read")
            continue
        off_centre = math.hypot(row["centre"][0] - centre[0], row["centre"][1] - centre[1])
        off_radius = abs(row["radius"] - radius)
        if off_centre <= TOLERANCE and off_radius <= TOLERANCE:
            matched += 1
        else:
            print(f"  {tag}: centre off {off_centre:.4f} ft, radius off {off_radius:.4f} ft")

        def point(angle):
            c, s = math.cos(angle), math.sin(angle)
            return (
                row["centre"][0] + row["radius"] * (c * row["x_axis"][0] + s * row["y_axis"][0]),
                row["centre"][1] + row["radius"] * (c * row["x_axis"][1] + s * row["y_axis"][1]),
            )

        for end in (point(row["angles"][0]), point(row["angles"][1])):
            ends.append(min(math.hypot(end[0] - p[0], end[1] - p[1]) for p in (first, last)))
    print(f"{len(revit)} walls with an arc axis in Revit's export, {len(probe)} arcs read")
    print(f"  {matched} with Revit's centre and radius within {TOLERANCE} ft")
    print(f"  {len(set(probe) - set(revit))} arcs read on walls Revit's axis draws otherwise")
    if ends:
        ends.sort()
        within = " ".join(f"{t} ft: {sum(1 for e in ends if e <= t)}" for t in (0.001, 0.01, 0.1, 1.0))
        print(f"  arc ends against Revit's axis ends: {len(ends)}; within {within}; max {ends[-1]:.3f} ft")


if __name__ == "__main__":
    main()

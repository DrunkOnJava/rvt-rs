#!/usr/bin/env python3
"""Print Revit's IFC flight for each spiral stair run the probe lists (#357).

Reads `examples/probe_357_spiral_runs.rs`'s output for the run ids and, for
each `IfcStairFlight` of Revit's export with that Tag, prints its body in
the site frame (Revit internal feet): its representation items, volume,
bounds, the distinct heights of its vertices, and, at each height, its
vertices' plan points, with the circle that best fits all its vertices.

Needs IfcOpenShell.

Usage:

    python3 tools/re/spiral_runs_vs_ifc.py <probe output> <revit.ifc> [--list]
"""

import collections
import math
import os
import sys

import ifcopenshell
import ifcopenshell.geom
import numpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from opening_boxes_vs_ifc import site_frame  # noqa: E402
from roof_bodies_vs_ifc import shapes_of  # noqa: E402


def circle_fit(points):
    a = numpy.array([[x, y, 1.0] for x, y in points])
    b = numpy.array([-(x * x + y * y) for x, y in points])
    (d, e, f), *_ = numpy.linalg.lstsq(a, b, rcond=None)
    cx, cy = -d / 2, -e / 2
    r = math.sqrt(max(cx * cx + cy * cy - f, 0.0))
    spread = max(abs(math.hypot(x - cx, y - cy) - r) for x, y in points)
    return cx, cy, r, spread


def main():
    runs = []
    for line in open(sys.argv[1], encoding="utf-8", errors="replace"):
        if line.startswith("run "):
            runs.append(line.split()[1])
    model = ifcopenshell.open(sys.argv[2])
    frame = site_frame(model)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    flights = {str(f.Tag): f for f in model.by_type("IfcStairFlight") if f.Tag}
    print(f"probe runs {len(runs)}: {' '.join(runs)}")
    for tag in runs:
        flight = flights.get(tag)
        if flight is None:
            print(f"run {tag}: no IfcStairFlight in Revit's export")
            continue
        items = []
        for rep in (flight.Representation.Representations if flight.Representation else []):
            items.append(f"{rep.RepresentationIdentifier}/{rep.RepresentationType}:" + ",".join(i.is_a() for i in rep.Items))
        points = []
        volume = 0.0
        for part in shapes_of(flight):
            try:
                shape = ifcopenshell.geom.create_shape(settings, part)
            except RuntimeError:
                continue
            verts = numpy.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
            if not len(verts):
                continue
            pts = frame(verts)
            faces = shape.geometry.faces
            for i in range(0, len(faces), 3):
                a, b, c = (pts[faces[i + k]] for k in range(3))
                volume += numpy.dot(a, numpy.cross(b, c)) / 6.0
            points.extend(tuple(round(float(c), 4) for c in p) for p in pts)
        unique = sorted(set(points))
        print(f"run {tag}: PredefinedType {flight.PredefinedType} items {' '.join(items)} vertices {len(unique)} volume {abs(volume):.6f}")
        if not unique:
            continue
        xs, ys, zs = zip(*unique)
        print(f"  bounds x {min(xs):.4f}..{max(xs):.4f} y {min(ys):.4f}..{max(ys):.4f} z {min(zs):.4f}..{max(zs):.4f}")
        cx, cy, r, spread = circle_fit([(x, y) for x, y, _ in unique])
        print(f"  plan circle centre ({cx:.4f}, {cy:.4f}) radius {r:.4f} spread {spread:.4f}")
        radii = collections.Counter(round(math.hypot(x - cx, y - cy), 3) for x, y, _ in unique)
        print(f"  radii {dict(sorted(radii.items()))}")
        levels = collections.defaultdict(list)
        for x, y, z in unique:
            levels[round(z, 3)].append((x, y))
        print(f"  heights {len(levels)}: {' '.join(str(z) for z in sorted(levels))}")
        if "--list" in sys.argv:
            for z in sorted(levels):
                angles = sorted(
                    (round(math.degrees(math.atan2(y - cy, x - cx)), 3), round(math.hypot(x - cx, y - cy), 4))
                    for x, y in levels[z]
                )
                print(f"  z {z}: {angles}")


if __name__ == "__main__":
    main()

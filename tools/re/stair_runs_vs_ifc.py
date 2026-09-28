#!/usr/bin/env python3
"""Score drawn stair runs against Revit's own geometry of each flight.

Research tool for `reports/element-framing/RE-92-stair-run-ends.md`.

`runs.json` is written by `examples/probe_re92_run_ends.rs --json`: each
drawn run's sketch origin, climb and across directions, width, and side
view as `[along, up]` feet. Revit's body for the flight with the same Tag
is meshed by IfcOpenShell, moved into Revit's internal frame through the
export's site placement, and put into the run's own frame. Two distances
are reported per run, in feet:

- drawn: the largest distance from a vertex of the drawn side view to the
  nearest of Revit's vertices, in the side-view plane;
- revit: the largest distance of one of Revit's vertices outside the drawn
  outline (0 when on or inside it), and whether all of them lie within the
  run's width.

Usage:

    python3 tools/re/stair_runs_vs_ifc.py <runs.json> <revit-export.ifc> [--ids ID,ID] [--dump]

Needs IfcOpenShell (tested with 0.8.5) and NumPy.
"""

import json
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.placement
import numpy as np

FEET = 0.3048


def site_frame(f):
    """Shared metres (IfcOpenShell world) -> Revit internal feet."""
    site = f.by_type("IfcSite")[0]
    mat = ifcopenshell.util.placement.get_local_placement(site.ObjectPlacement)
    ox, oy, oz = mat[0][3], mat[1][3], mat[2][3]
    c, s = mat[0][0], mat[1][0]

    def points(v):
        dx, dy = v[:, 0] - ox, v[:, 1] - oy
        return np.stack(
            [(c * dx + s * dy) / FEET, (-s * dx + c * dy) / FEET, (v[:, 2] - oz) / FEET], axis=1
        )

    return points


def vertices(element, frame):
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    try:
        shape = ifcopenshell.geom.create_shape(settings, element)
    except Exception:
        return None
    verts = np.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
    return frame(verts) if len(verts) else None


def segment_distance(p, a, b):
    ab = b - a
    t = np.clip(np.dot(p - a, ab) / max(np.dot(ab, ab), 1e-18), 0.0, 1.0)
    return float(np.linalg.norm(p - (a + t * ab)))


def inside(p, ring):
    x, y = p
    hit = False
    for (x1, y1), (x2, y2) in zip(ring, np.roll(ring, -1, axis=0)):
        if (y1 > y) != (y2 > y) and x < x1 + (y - y1) * (x2 - x1) / (y2 - y1):
            hit = not hit
    return hit


def outside_distance(p, ring):
    edge = min(segment_distance(p, a, b) for a, b in zip(ring, np.roll(ring, -1, axis=0)))
    return 0.0 if inside(p, ring) or edge < 1e-9 else edge


def main():
    args = sys.argv[1:]
    if len(args) < 2:
        sys.exit(__doc__)
    runs = json.load(open(args[0]))
    revit = ifcopenshell.open(args[1])
    wanted = None
    if "--ids" in args:
        wanted = {int(x) for x in args[args.index("--ids") + 1].split(",")}
    dump = "--dump" in args
    frame = site_frame(revit)
    flights = {}
    for element in revit.by_type("IfcStairFlight"):
        if element.Tag:
            flights.setdefault(element.Tag, element)
    print(f"{'run':>8} {'type':>8}  {'drawn':>8}  {'revit':>8}  across  verts")
    for run in runs:
        if wanted and run["id"] not in wanted:
            continue
        element = flights.get(str(run["id"]))
        verts = vertices(element, frame) if element else None
        if verts is None:
            print(f"{run['id']:>8} {run['type']:>8}  no Revit flight with this Tag")
            continue
        origin = np.array(run["origin"])
        climb = np.array(run["climb"])
        across = np.array(run["across"])
        rel = verts - origin
        side = np.stack([rel[:, :2] @ climb, rel[:, 2]], axis=1)
        width = rel[:, :2] @ across
        ring = np.array(run["profile"])
        drawn = max(float(np.min(np.linalg.norm(side - p, axis=1))) for p in ring)
        revit_out = max(outside_distance(p, ring) for p in side)
        within = bool(np.all((width > -1e-4) & (width < run["width"] + 1e-4)))
        print(
            f"{run['id']:>8} {run['type']:>8}  {drawn:8.5f}  {revit_out:8.5f}  "
            f"{'yes' if within else 'NO ':>6}  {len(verts)}"
        )
        if dump:
            print("    drawn:", [[round(u, 4), round(z, 4)] for u, z in ring])
            unique = sorted({(round(u, 4), round(z, 4)) for u, z in side})
            print("    revit:", unique)


if __name__ == "__main__":
    main()

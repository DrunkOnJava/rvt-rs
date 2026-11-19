#!/usr/bin/env python3
"""Where each body's vertices leave the other's surface, per stair flight (#2094).

    python3 tools/re/far_vertices_vs_ifc.py <rvt-rs.ifc> <revit.ifc>

Needs IfcOpenShell and numpy. A temporary diagnostic for #2094, run by Measure as an IFC
scorer: not part of any scorer list.

For every `IfcStairFlight` both exports hold (same `Tag`), meshed as
`stair_bodies_vs_ifc.py` meshes it (each file's own site frame), this lists:

- Revit's vertices more than 0.001 ft from rvt-rs's surface, and rvt-rs's more than
  0.01 ft from Revit's (the bars `mesh_bodies_vs_ifc.py` scores), the 12 farthest of each;
- for each: its distance, where it sits in the flight's box (each axis as a fraction from
  the low face, 0, to the high one, 1), the facing of the nearest triangle of the other
  body (`up`, `down`, `side`, or `slope` with its angle from the vertical), and whether it
  lies on a crease of its own body (two triangles meeting at it more than 10 degrees
  apart, a boundary between faces) or inside a smooth stretch (`smooth`).

Only flights with such a vertex are listed. Positions are fractions of a flight's own
box, not model coordinates.
"""

import math
import os
import sys

import numpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from stair_bodies_vs_ifc import bodies  # noqa: E402

REVIT_BAR = 0.001
OURS_BAR = 0.01
LISTED = 12
CREASE_DEGREES = 10.0


def nearest(points, triangles):
    """Each point's distance from the nearest of `triangles`, and that triangle's index."""
    a, b, c = (numpy.array([tri[k] for tri in triangles]) for k in range(3))
    normal = numpy.cross(b - a, c - a)
    area = numpy.linalg.norm(normal, axis=1)
    unit = normal / numpy.where(area > 0, area, 1)[:, None]

    def segment(p, u, v):
        uv = v - u
        length2 = numpy.einsum("ij,ij->i", uv, uv)
        t = numpy.clip(numpy.einsum("ij,ij->i", p - u, uv) / numpy.where(length2 > 0, length2, 1), 0, 1)
        return numpy.linalg.norm(p - (u + t[:, None] * uv), axis=1)

    out = []
    for point in points:
        p = numpy.array(point)
        height = numpy.einsum("ij,ij->i", p - a, unit)
        foot = p - height[:, None] * unit
        inside = area > 0
        for u, v in ((a, b), (b, c), (c, a)):
            inside &= numpy.einsum("ij,ij->i", numpy.cross(v - u, foot - u), normal) >= 0
        edge = numpy.minimum.reduce([segment(p, a, b), segment(p, b, c), segment(p, c, a)])
        d = numpy.where(inside, numpy.abs(height), edge)
        i = int(numpy.argmin(d))
        out.append((float(d[i]), unit[i]))
    return out


def facing(unit):
    z = float(unit[2])
    if z > 0.999:
        return "up"
    if z < -0.999:
        return "down"
    if abs(z) < 0.001:
        return "side"
    return f"slope{math.degrees(math.acos(min(1.0, abs(z)))):.0f}"


def creases(triangles):
    """Vertex (rounded) -> the largest angle, degrees, between two triangles meeting there."""
    normals = {}
    for tri in triangles:
        a, b, c = (numpy.array(p) for p in tri)
        n = numpy.cross(b - a, c - a)
        length = numpy.linalg.norm(n)
        if length == 0:
            continue
        n = n / length
        for p in tri:
            normals.setdefault(tuple(round(x, 6) for x in p), []).append(n)
    out = {}
    for key, ns in normals.items():
        worst = 0.0
        for i in range(len(ns)):
            for j in range(i + 1, len(ns)):
                worst = max(worst, math.degrees(math.acos(max(-1.0, min(1.0, float(ns[i] @ ns[j]))))))
        out[key] = worst
    return out


def listed(points, triangles_self, triangles_other, bar, low, high):
    """The vertices of one body farther than `bar` from the other's surface, farthest first."""
    found = nearest(points, triangles_other)
    crease = creases(triangles_self)
    rows = []
    for point, (distance, unit) in zip(points, found):
        if distance <= bar:
            continue
        span = [h - lo if h > lo else 1.0 for lo, h in zip(low, high)]
        where = ",".join(f"{(p - lo) / s:.2f}" for p, lo, s in zip(point, low, span))
        angle = crease.get(tuple(round(x, 6) for x in point), 0.0)
        kind = "crease" if angle > CREASE_DEGREES else "smooth"
        rows.append((distance, where, facing(unit), kind))
    rows.sort(reverse=True)
    return rows


def main():
    args = [arg for arg in sys.argv[1:] if not arg.startswith("--")]
    if len(args) != 2:
        print(__doc__)
        return 2
    ours = bodies(args[0], "IfcStairFlight")
    ref = bodies(args[1], "IfcStairFlight")
    both = sorted(set(ours) & set(ref), key=lambda t: (len(t), t))
    print(f"far vertices: {len(both)} flights in both exports")
    for tag in both:
        (our_vertices, _, our_triangles), (ref_vertices, _, ref_triangles) = ours[tag], ref[tag]
        if not our_triangles or not ref_triangles:
            continue
        everything = our_vertices + ref_vertices
        low = [min(v[k] for v in everything) for k in range(3)]
        high = [max(v[k] for v in everything) for k in range(3)]
        revit = listed(ref_vertices, ref_triangles, our_triangles, REVIT_BAR, low, high)
        mine = listed(our_vertices, our_triangles, ref_triangles, OURS_BAR, low, high)
        if not revit and not mine:
            continue
        print(
            f"far vertices: {tag}: Revit's beyond {REVIT_BAR} ft of ours {len(revit)} of {len(ref_vertices)}; "
            f"ours beyond {OURS_BAR} ft of Revit's {len(mine)} of {len(our_vertices)}"
        )
        for who, rows in (("Revit", revit), ("rvt-rs", mine)):
            kinds = {}
            for _, _, face, kind in rows:
                kinds[(face, kind)] = kinds.get((face, kind), 0) + 1
            if kinds:
                summary = "; ".join(f"{face} {kind} {n}" for (face, kind), n in sorted(kinds.items()))
                print(f"far vertices:   {who}'s, by the other's facing and own crease: {summary}")
            for distance, where, face, kind in rows[:LISTED]:
                print(f"far vertices:     {who} {distance:.5f} ft at {where} facing {face} {kind}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

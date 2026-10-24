#!/usr/bin/env python3
"""Score rvt-rs stair bodies against Revit's own IFC export, per flight (#357).

For every `IfcStairFlight` both exports hold (same `Tag`, the run's
ElementId), and every `IfcStair` with its aggregated parts, this meshes the
body in world space with IfcOpenShell and compares, in feet, as
`roof_bodies_vs_ifc.py` does for roofs:

- the volume of the two meshes;
- how far each vertex of Revit's body lies from the nearest vertex of
  rvt-rs's, the worst of them;
- how far each vertex of Revit's body lies from rvt-rs's surface.

A flight drawn as its record box has the box's volume and corners, so it
misses on both; one drawn as its treads and risers matches on both or on
neither. The surface test does not tell a box from a flight: a straight
flight's vertices all lie on its two side faces, which are the box's sides.
On `main` every flight of the R2025 stair models (13 to 17) is its box,
with 4 to 57 ft3 of Revit's against 237 to 684 ft3 (Measure 40140290382).

Needs IfcOpenShell (`pip install ifcopenshell`).

Usage:

    python3 tools/re/stair_bodies_vs_ifc.py <rvt-rs.ifc> <revit.ifc>

Every flight and stair both hold gets a line, matching or not.
"""

import argparse
import math
import os
import sys

import ifcopenshell
import ifcopenshell.geom

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from roof_bodies_vs_ifc import FT, VERTEX_TOLERANCE_FT, VOLUME_TOLERANCE, farthest, shapes_of, surface_distances  # noqa: E402

CLASSES = ("IfcStairFlight", "IfcStair")


def bodies(path, cls):
    """Tag -> (unique vertices, volume, triangles) of each `cls`, in feet and cubic feet."""
    model = ifcopenshell.open(path)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    out = {}
    for element in model.by_type(cls):
        if not element.Tag or element.is_a() != cls:
            continue
        vertices = set()
        triangles = []
        volume = 0.0
        meshed = False
        for part in shapes_of(element):
            try:
                shape = ifcopenshell.geom.create_shape(settings, part)
            except RuntimeError:
                continue
            verts = shape.geometry.verts
            faces = shape.geometry.faces
            points = [(verts[i] / FT, verts[i + 1] / FT, verts[i + 2] / FT) for i in range(0, len(verts), 3)]
            if not points:
                continue
            meshed = True
            vertices.update(tuple(round(c, 6) for c in p) for p in points)
            for i in range(0, len(faces), 3):
                a, b, c = (points[faces[i + k]] for k in range(3))
                triangles.append((a, b, c))
                volume += (
                    a[0] * (b[1] * c[2] - b[2] * c[1])
                    - a[1] * (b[0] * c[2] - b[2] * c[0])
                    + a[2] * (b[0] * c[1] - b[1] * c[0])
                ) / 6.0
        if meshed:
            out[element.Tag] = (sorted(vertices), abs(volume), triangles)
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("ours")
    ap.add_argument("reference")
    args = ap.parse_args()
    for cls in CLASSES:
        ours = bodies(args.ours, cls)
        ref = bodies(args.reference, cls)
        both = sorted(set(ours) & set(ref), key=lambda t: (len(t), t))
        print(f"{cls} in Revit's export with a body: {len(ref)}; rvt-rs writes one for {len(both)} of them")
        if not both:
            continue
        ok = on_surface = 0
        rows = []
        for tag in both:
            (our_vertices, our_volume, our_triangles), (ref_vertices, ref_volume, _) = ours[tag], ref[tag]
            distance = farthest(ref_vertices, our_vertices)
            surface = surface_distances(ref_vertices, our_triangles) if our_triangles else [math.inf]
            on_surface += max(surface) <= VERTEX_TOLERANCE_FT
            relative = abs(our_volume - ref_volume) / ref_volume if ref_volume > 0 else math.inf
            matches = distance <= VERTEX_TOLERANCE_FT and relative <= VOLUME_TOLERANCE
            ok += matches
            rows.append((tag, matches, len(ref_vertices), distance, max(surface), our_volume, ref_volume))
        print(
            f"  vertices within {VERTEX_TOLERANCE_FT} ft and volume within {VOLUME_TOLERANCE:g} of Revit's: "
            f"{ok} of {len(both)}"
        )
        print(f"  every vertex of Revit's body within {VERTEX_TOLERANCE_FT} ft of rvt-rs's surface: {on_surface} of {len(both)}")
        for tag, matches, count, distance, surface, our_volume, ref_volume in rows:
            print(
                f"    {tag}: {'match' if matches else 'differs'}, Revit {count} vertices, "
                f"farthest {distance:.3g} ft, from rvt-rs's surface at most {surface:.3g} ft, "
                f"volume rvt-rs {our_volume:.6f} ft3, Revit {ref_volume:.6f} ft3"
            )


if __name__ == "__main__":
    main()

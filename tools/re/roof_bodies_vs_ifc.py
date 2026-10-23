#!/usr/bin/env python3
"""Score rvt-rs roof bodies against Revit's own IFC export, per roof (#356).

Research tool for `reports/element-framing/RE-199-roof-planes.md`. For every
`IfcRoof` both exports hold (same `Tag`, the ElementId), this meshes the roof's
body in world space with IfcOpenShell, the roof's own representation and its
aggregated parts' together, and compares, in feet:

- the volume of the two meshes;
- how far each vertex of Revit's body lies from the nearest vertex of
  rvt-rs's, the worst of them;
- how far each vertex of Revit's body lies from rvt-rs's surface. A body
  swept along a curve or a straight run needs no vertices between its ends,
  where Revit's mesh has them, so this is the test for one (RE-211).

A level block as tall as the roof's record box has the box's volume and its
corners, so a sloped roof drawn as one misses on both; a roof drawn as its
planes matches on both or on neither.

Needs IfcOpenShell (`pip install ifcopenshell`).

Usage:

    python3 tools/re/roof_bodies_vs_ifc.py <rvt-rs.ifc> <revit.ifc>

Every roof both hold gets a line, matching or not.
"""

import argparse
import math

import ifcopenshell
import ifcopenshell.geom
import numpy

FT = 0.3048
VERTEX_TOLERANCE_FT = 0.001
VOLUME_TOLERANCE = 1e-4


def shapes_of(element):
    """The element itself and its aggregated parts that carry a body."""
    out = [element] if element.Representation else []
    for rel in getattr(element, "IsDecomposedBy", None) or []:
        out.extend(part for part in rel.RelatedObjects if part.Representation)
    return out


def roof_bodies(path):
    """Tag -> (unique vertices, volume, triangles), in feet and cubic feet."""
    model = ifcopenshell.open(path)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    out = {}
    for roof in model.by_type("IfcRoof"):
        if not roof.Tag:
            continue
        vertices = set()
        triangles = []
        volume = 0.0
        meshed = False
        for element in shapes_of(roof):
            try:
                shape = ifcopenshell.geom.create_shape(settings, element)
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
            out[roof.Tag] = (sorted(vertices), abs(volume), triangles)
    return out


def farthest(reference, ours):
    """The largest distance from a vertex of `reference` to its nearest in `ours`."""
    return max(min(math.dist(p, q) for q in ours) for p in reference)


def segment_distances(p, a, b):
    """Each segment a-b's distance from the point p."""
    ab = b - a
    length2 = numpy.einsum("ij,ij->i", ab, ab)
    t = numpy.clip(numpy.einsum("ij,ij->i", p - a, ab) / numpy.where(length2 > 0, length2, 1), 0, 1)
    return numpy.linalg.norm(p - (a + t[:, None] * ab), axis=1)


def surface_distances(points, triangles):
    """Each point's distance from the nearest of `triangles`."""
    a, b, c = (numpy.array([tri[k] for tri in triangles]) for k in range(3))
    normal = numpy.cross(b - a, c - a)
    area = numpy.linalg.norm(normal, axis=1)
    unit = normal / numpy.where(area > 0, area, 1)[:, None]
    out = []
    for point in points:
        p = numpy.array(point)
        height = numpy.einsum("ij,ij->i", p - a, unit)
        foot = p - height[:, None] * unit
        inside = area > 0
        for u, v in ((a, b), (b, c), (c, a)):
            inside &= numpy.einsum("ij,ij->i", numpy.cross(v - u, foot - u), normal) >= 0
        edge = numpy.minimum.reduce([segment_distances(p, a, b), segment_distances(p, b, c), segment_distances(p, c, a)])
        out.append(float(numpy.min(numpy.where(inside, numpy.abs(height), edge))))
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("ours")
    ap.add_argument("reference")
    args = ap.parse_args()
    ours = roof_bodies(args.ours)
    ref = roof_bodies(args.reference)
    both = sorted(set(ours) & set(ref), key=lambda t: (len(t), t))
    print(f"roofs in Revit's export with a body: {len(ref)}; rvt-rs writes one for {len(both)} of them")
    if not both:
        return
    ok = 0
    rows = []
    on_surface = 0
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
        f"vertices within {VERTEX_TOLERANCE_FT} ft and volume within {VOLUME_TOLERANCE:g} of Revit's: "
        f"{ok} of {len(both)}"
    )
    print(f"every vertex of Revit's body within {VERTEX_TOLERANCE_FT} ft of rvt-rs's surface: {on_surface} of {len(both)}")
    for tag, matches, count, distance, surface, our_volume, ref_volume in rows:
        print(
            f"  {tag}: {'match' if matches else 'differs'}, Revit {count} vertices, "
            f"farthest {distance:.3g} ft, from rvt-rs's surface at most {surface:.3g} ft, "
            f"volume rvt-rs {our_volume:.6f} ft3, Revit {ref_volume:.6f} ft3"
        )


if __name__ == "__main__":
    main()

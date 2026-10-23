#!/usr/bin/env python3
"""Score rvt-rs roof bodies against Revit's own IFC export, per roof (#356).

Research tool for `reports/element-framing/RE-199-roof-planes.md`. For every
`IfcRoof` both exports hold (same `Tag`, the ElementId), this meshes the roof's
body in world space with IfcOpenShell, the roof's own representation and its
aggregated parts' together, and compares, in feet:

- the volume of the two meshes;
- how far each vertex of Revit's body lies from the nearest vertex of
  rvt-rs's, the worst of them.

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
    """Tag -> (unique vertices, volume), in feet and cubic feet."""
    model = ifcopenshell.open(path)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    out = {}
    for roof in model.by_type("IfcRoof"):
        if not roof.Tag:
            continue
        vertices = set()
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
                volume += (
                    a[0] * (b[1] * c[2] - b[2] * c[1])
                    - a[1] * (b[0] * c[2] - b[2] * c[0])
                    + a[2] * (b[0] * c[1] - b[1] * c[0])
                ) / 6.0
        if meshed:
            out[roof.Tag] = (sorted(vertices), abs(volume))
    return out


def farthest(reference, ours):
    """The largest distance from a vertex of `reference` to its nearest in `ours`."""
    return max(min(math.dist(p, q) for q in ours) for p in reference)


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
    for tag in both:
        (our_vertices, our_volume), (ref_vertices, ref_volume) = ours[tag], ref[tag]
        distance = farthest(ref_vertices, our_vertices)
        relative = abs(our_volume - ref_volume) / ref_volume if ref_volume > 0 else math.inf
        matches = distance <= VERTEX_TOLERANCE_FT and relative <= VOLUME_TOLERANCE
        ok += matches
        rows.append((tag, matches, len(ref_vertices), distance, our_volume, ref_volume))
    print(
        f"vertices within {VERTEX_TOLERANCE_FT} ft and volume within {VOLUME_TOLERANCE:g} of Revit's: "
        f"{ok} of {len(both)}"
    )
    for tag, matches, count, distance, our_volume, ref_volume in rows:
        print(
            f"  {tag}: {'match' if matches else 'differs'}, Revit {count} vertices, "
            f"farthest {distance:.3g} ft, volume rvt-rs {our_volume:.6f} ft3, Revit {ref_volume:.6f} ft3"
        )


if __name__ == "__main__":
    main()

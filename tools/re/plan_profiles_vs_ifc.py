#!/usr/bin/env python3
"""Score sketched plan profiles (slabs, shading devices, roofs) against Revit's IFC4.

Research tool for `reports/element-framing/RE-95-zero-length-sketch-lines.md`
(#233). Both files are meshed by IfcOpenShell and moved into Revit's
internal feet through each export's own site placement. For each element of
the given classes whose Tag is in both files, it compares the upward-facing
faces of the two bodies:

- plan area: the area of the element's top surface, the plan union of its
  upward faces (Revit writes a layered floor, roof or ceiling as one solid
  per layer, stacked, each with the element's outline, so a sum would count
  each layer; the union counts the outline once, and keeps a sloped top
  whole, where its highest faces alone are a part of it, #824);
- vertices: the largest distance from a vertex of one file's upward faces
  to the nearest vertex of the other's, both ways, in plan;
- outline: the largest distance from a vertex of one file's top surface to
  the other's top-surface outline, both ways, in plan. Revit splits an
  edge wherever the element bounding it changes, so a room's outline can
  be the same shape with more vertices.

Where at most ten elements are off in area or outline, each is listed by
Tag; `--list N` lists the N farthest besides.

Usage:

    python3 tools/re/plan_profiles_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc> [--class IfcSlab,IfcShadingDevice] [--list N] [--match centroid]

`--match centroid` pairs each rvt-rs element with the Revit one whose top
surface's centroid is nearest, for a file whose rooms do not carry Revit's
GlobalId (Revit 2023).

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins), NumPy and Shapely.
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.geom
import numpy as np

from ifc_common import site_frame
import shapely
from shapely.geometry import Polygon


def tops(path, classes):
    """Tag -> (upward plan area, upward-face vertices in plan), every
    element with the Tag (every piece of a sketch) taken together."""
    f = ifcopenshell.open(path)
    frame = site_frame(f)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    out = {}
    for cls in classes:
        try:
            elements = f.by_type(cls)
        except RuntimeError:
            # Not in this file's schema (IfcShadingDevice in IFC2X3).
            continue
        for element in elements:
            # IfcSpace declares no Tag; Revit's GlobalId, which rvt-rs
            # derives the same way, keys it instead.
            tag = getattr(element, "Tag", None) or element.GlobalId
            if not tag or not element.Representation:
                continue
            try:
                shape = ifcopenshell.geom.create_shape(settings, element)
            except Exception:
                continue
            verts = np.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
            faces = np.array(shape.geometry.faces, dtype=int).reshape(-1, 3)
            if not len(verts) or not len(faces):
                continue
            v = frame(verts)
            a, b, c = v[faces[:, 0]], v[faces[:, 1]], v[faces[:, 2]]
            normal = np.cross(b - a, c - a)
            length = np.linalg.norm(normal, axis=1)
            up = (length > 1e-12) & (normal[:, 2] > 0.9 * length)
            points = v[np.unique(faces[up].ravel())][:, :2]
            triangles = [
                Polygon([tuple(p[:2]) for p in tri]).buffer(1e-7)
                for tri in zip(a[up], b[up], c[up])
            ]
            surface = shapely.union_all(triangles) if triangles else Polygon()
            # A sketch of several pieces is several elements with one Tag.
            if tag in out:
                _, held_points, _, held_surface = out[tag]
                points = np.vstack([held_points, points])
                surface = shapely.union_all([held_surface, surface])
            out[tag] = (float(surface.area), points, element.is_a(), surface)
    return out


def spread(p, q):
    if not len(p) or not len(q):
        return float("inf")
    d = np.linalg.norm(p[:, None, :] - q[None, :, :], axis=2)
    return float(max(d.min(axis=1).max(), d.min(axis=0).max()))


def outline_spread(p, q_surface, q, p_surface):
    if not len(p) or not len(q) or q_surface.is_empty or p_surface.is_empty:
        return float("inf")
    far = lambda pts, surface: max(surface.boundary.distance(shapely.Point(x, y)) for x, y in pts)
    return float(max(far(p, q_surface), far(q, p_surface)))


def main(argv):
    args = argv[1:]
    classes = ["IfcSlab", "IfcShadingDevice"]
    listing = 0
    if "--class" in args:
        i = args.index("--class")
        classes = args[i + 1].split(",")
        del args[i : i + 2]
    if "--list" in args:
        i = args.index("--list")
        listing = int(args[i + 1])
        del args[i : i + 2]
    by_centroid = False
    if "--match" in args:
        i = args.index("--match")
        by_centroid = args[i + 1] == "centroid"
        del args[i : i + 2]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    ours, revit = tops(args[0], classes), tops(args[1], classes)
    if by_centroid and revit:
        paired = {}
        for held in ours.values():
            nearest = min(
                revit.items(), key=lambda item: item[1][3].centroid.distance(held[3].centroid)
            )[0]
            paired[nearest] = held
        ours = paired
    rows = []
    for tag, (area, points, cls, surface) in revit.items():
        if tag in ours and area > 0:
            o_area, o_points, _, o_surface = ours[tag]
            rows.append(
                (
                    abs(o_area - area) / area,
                    spread(o_points, points),
                    tag,
                    cls,
                    area,
                    o_area,
                    outline_spread(o_points, surface, points, o_surface),
                )
            )
    print(f"elements of {','.join(classes)} in both: {len(rows)}")
    counts = collections.Counter()
    for rel, dist, _, cls, _, _, edge in rows:
        counts[(cls, "area within 0.1%" if rel <= 1e-3 else "area off")] += 1
        counts[(cls, "vertices within 0.01 ft" if dist <= 0.01 else "vertices off")] += 1
        counts[(cls, "outline within 0.01 ft" if edge <= 0.01 else "outline off")] += 1
    for (cls, what), count in sorted(counts.items()):
        print(f"  {cls} {what}: {count}")
    off = [row for row in rows if row[0] > 1e-3 or row[6] > 0.01]
    shown = sorted(off, reverse=True) if len(off) <= 10 else []
    for rel, dist, tag, cls, area, o_area, edge in shown + sorted(rows, reverse=True)[:listing]:
        print(
            f"  {tag} {cls}: area {o_area:.2f} vs {area:.2f} ft2 ({rel:.2%}),"
            f" vertices {dist:.3f} ft, outline {edge:.3f} ft"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

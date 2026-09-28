#!/usr/bin/env python3
"""Score sketched plan profiles (slabs, shading devices, roofs) against Revit's IFC4.

Research tool for `reports/element-framing/RE-95-zero-length-sketch-lines.md`
(#233). Both files are meshed by IfcOpenShell and moved into Revit's
internal feet through each export's own site placement. For each element of
the given classes whose Tag is in both files, it compares the upward-facing
faces of the two bodies:

- plan area: the area of the upward faces projected on plan;
- vertices: the largest distance from a vertex of one file's upward faces
  to the nearest vertex of the other's, both ways, in plan.

Usage:

    python3 tools/re/plan_profiles_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc> [--class IfcSlab,IfcShadingDevice] [--list N]

Needs IfcOpenShell (tested with 0.8.5) and NumPy.
"""

import collections
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


def tops(path, classes):
    """Tag -> (upward plan area, upward-face vertices in plan), every
    element with the Tag (every piece of a sketch) taken together."""
    f = ifcopenshell.open(path)
    frame = site_frame(f)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    out = {}
    for cls in classes:
        for element in f.by_type(cls):
            tag = getattr(element, "Tag", None)
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
            area = float(0.5 * normal[up, 2].sum())
            points = v[np.unique(faces[up].ravel())][:, :2]
            # A sketch of several pieces is several elements with one Tag.
            if tag in out:
                held_area, held_points, _ = out[tag]
                area, points = held_area + area, np.vstack([held_points, points])
            out[tag] = (area, points, element.is_a())
    return out


def spread(p, q):
    if not len(p) or not len(q):
        return float("inf")
    d = np.linalg.norm(p[:, None, :] - q[None, :, :], axis=2)
    return float(max(d.min(axis=1).max(), d.min(axis=0).max()))


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
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    ours, revit = tops(args[0], classes), tops(args[1], classes)
    rows = []
    for tag, (area, points, cls) in revit.items():
        if tag in ours and area > 0:
            o_area, o_points, _ = ours[tag]
            rows.append((abs(o_area - area) / area, spread(o_points, points), tag, cls, area, o_area))
    print(f"elements of {','.join(classes)} in both: {len(rows)}")
    counts = collections.Counter()
    for rel, dist, _, cls, _, _ in rows:
        counts[(cls, "area within 0.1%" if rel <= 1e-3 else "area off")] += 1
        counts[(cls, "vertices within 0.01 ft" if dist <= 0.01 else "vertices off")] += 1
    for (cls, what), count in sorted(counts.items()):
        print(f"  {cls} {what}: {count}")
    for rel, dist, tag, cls, area, o_area in sorted(rows, reverse=True)[:listing]:
        print(f"  {tag} {cls}: area {o_area:.2f} vs {area:.2f} ft2 ({rel:.2%}), vertices {dist:.3f} ft")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

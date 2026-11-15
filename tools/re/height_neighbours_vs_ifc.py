#!/usr/bin/env python3
"""What stands at a height near one element's end in Revit's export (#1474).

For the BIM4LCA office's wall 2056705, whose end Revit's export draws to
179.298 ft over a strip beside wall 1493966's face, it lists every element of
Revit's export with a vertex within 0.005 ft of that height and within 3 ft
in plan of the wall's box, with its class, Tag, Name and height range, and the
material layers Revit's export gives walls 1493966 and 2056705. Heights are
Revit internal feet (ifc_common.site_frame).

Usage:

    python3 tools/re/height_neighbours_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>

Only Revit's export is read; the first argument is ignored, as Measure passes it.
"""

import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import ifcopenshell  # noqa: E402
import ifcopenshell.geom  # noqa: E402
import ifcopenshell.util.element as ue  # noqa: E402
from ifc_common import site_frame  # noqa: E402

WALL = "2056705"
PARTNER = "1493966"
HEIGHTS = (179.298, 182.087, 183.071)
NEAR = 3.0
TOL = 0.005


def main():
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    f = ifcopenshell.open(sys.argv[2])
    to_feet = site_frame(f)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    by_tag = {e.Tag: e for e in f.by_type("IfcElement") if getattr(e, "Tag", None)}
    wall = by_tag.get(WALL)
    if wall is None:
        print(f"wall {WALL} not in Revit's export")
        return 0
    shapes = {}

    faces = {}

    def edges(element):
        if element.id() not in faces:
            verts(element)
        return faces.get(element.id())

    def verts(element):
        if element.id() not in shapes:
            try:
                shape = ifcopenshell.geom.create_shape(settings, element)
                shapes[element.id()] = to_feet(np.array(shape.geometry.verts, dtype=float).reshape(-1, 3))
                faces[element.id()] = np.array(shape.geometry.faces, dtype=int).reshape(-1, 3)
            except Exception:  # noqa: BLE001 - an element IfcOpenShell cannot mesh is skipped
                shapes[element.id()] = None
        return shapes[element.id()]

    w = verts(wall)
    low, high = w.min(axis=0), w.max(axis=0)
    print(f"wall {WALL} {wall.Name}: x {low[0]:.3f}..{high[0]:.3f} y {low[1]:.3f}..{high[1]:.3f} z {low[2]:.3f}..{high[2]:.3f}")
    for tag in (WALL, PARTNER):
        element = by_tag.get(tag)
        if element is None:
            continue
        material = ue.get_material(element) or (ue.get_type(element) and ue.get_material(ue.get_type(element)))
        if material is not None and material.is_a("IfcMaterialLayerSetUsage"):
            material = material.ForLayerSet
        layers = []
        if material is not None and material.is_a("IfcMaterialLayerSet"):
            layers = [(layer.Material.Name if layer.Material else "-", round(layer.LayerThickness, 5)) for layer in material.MaterialLayers]
        print(f"layers of {tag} ({element.ObjectType}): {layers} (material {material.is_a() if material is not None else None})")
        v = verts(element)
        if v is not None:
            print(f"  distinct vertex heights of {tag}: {sorted(set(round(float(z), 3) for z in v[:, 2]))[:40]}")
    for height in HEIGHTS:
        print(f"elements with a horizontal edge at {height:.3f} ft (+-{TOL}) within {NEAR} ft of the wall in plan:")
        for element in f.by_type("IfcElement"):
            if element.is_a("IfcOpeningElement"):
                continue
            v = verts(element)
            if v is None or not len(v):
                continue
            tri = edges(element)
            if tri is None or not len(tri):
                continue
            # a horizontal edge at the height whose span passes the region in plan
            count = 0
            for a, b in ((0, 1), (1, 2), (2, 0)):
                pa, pb = v[tri[:, a]], v[tri[:, b]]
                at = (np.abs(pa[:, 2] - height) <= TOL) & (np.abs(pb[:, 2] - height) <= TOL)
                lo = np.minimum(pa, pb)
                hi = np.maximum(pa, pb)
                over = at & (hi[:, 0] > low[0] - NEAR) & (lo[:, 0] < high[0] + NEAR) & (hi[:, 1] > low[1] - NEAR) & (lo[:, 1] < high[1] + NEAR)
                count += int(over.sum())
            if count:
                print(f"  {element.is_a()} {element.Tag} {element.Name!r} z {v[:, 2].min():.3f}..{v[:, 2].max():.3f} ({count} horizontal edges)")
    return 0


if __name__ == "__main__":
    sys.exit(main())

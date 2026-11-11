#!/usr/bin/env python3
"""Revit's volume of each roof rvt-rs draws from its saved mesh, over the mesh's, against the cosine of
the roof's slope (#1656, RE-415).

A roof is the slab between two parallel planes, and its saved mesh puts them the roof type's thickness
apart perpendicular to the slope. For each `IfcRoof` and `IfcSlab ROOF` whose `BodySource` is
`saved_graphics_mesh` and that Revit's export also holds (same `Tag`), this prints the volume of the
mesh, the volume of Revit's body over its distinct triangles (a body Revit writes with every face
twice counts once, as `volume_readings.py` counts it), their ratio, the cosine of the slope (the mean
|normal z| of the mesh's faces that face up or down, weighted by area) and the ratio over that
cosine. A roof whose Revit volume is the mesh's times the cosine of its slope reads 1.0000 in the last
column; a flat roof has a cosine of 1 and reads its own ratio there.

Usage:

    python3 tools/re/roof_volume_vs_slope.py <rvt-rs.ifc> <revit.ifc>

Needs IfcOpenShell and numpy, as `mesh_bodies_vs_ifc.py` does.
"""

import os
import sys

import ifcopenshell
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mesh_bodies_vs_ifc as meshes  # noqa: E402
from roof_primitives_vs_ifc import distinct  # noqa: E402
from saved_meshes_vs_ifc import body_sources  # noqa: E402


def is_roof(element):
    return element.is_a("IfcRoof") or (element.is_a("IfcSlab") and getattr(element, "PredefinedType", None) == "ROOF")


def slope_cosine(triangles):
    """The mean |normal z| of the faces of `triangles` that face up or down, weighted by area."""
    cross = np.cross(triangles[:, 1] - triangles[:, 0], triangles[:, 2] - triangles[:, 0])
    area = np.linalg.norm(cross, axis=1)
    keep = area > 0
    z = np.abs(cross[keep, 2]) / area[keep]
    flat = z > 0.5
    return float((z[flat] * area[keep][flat]).sum() / area[keep][flat].sum()) if flat.any() else None


def main():
    if len(sys.argv) != 3:
        print(__doc__)
        return 2
    ours_path, ref_path = sys.argv[1:3]
    model = ifcopenshell.open(ours_path)
    names = {
        e.Tag: e.Name
        for e in model.by_type("IfcElement")
        if e.Tag and not e.is_a("IfcOpeningElement") and is_roof(e) and body_sources(e) == meshes.SOURCE
    }
    ours = meshes.meshes(ours_path, tags=set(names), role="ours", model=model)
    ref = meshes.meshes(ref_path, tags=set(ours), role="ref")
    print(f"roofs drawn from a saved mesh: {len(names)}; in Revit's export with a body: {len(set(ours) & set(ref))}")
    print("tag name: volume of the mesh, of Revit's body (distinct triangles), ratio, cos(slope), ratio / cos(slope)")
    for tag in sorted(set(ours) & set(ref), key=lambda t: (len(t), t)):
        mesh_volume = meshes.volume(ours[tag][2])
        revit_volume = meshes.volume(distinct(ref[tag][2]))
        ratio = revit_volume / mesh_volume
        cosine = slope_cosine(ours[tag][2])
        quotient = f"{ratio / cosine:.5f}" if cosine else "-"
        print(
            f"{tag} {names[tag]!r}: {mesh_volume:.4f} {revit_volume:.4f} ratio {ratio:.5f}"
            f" cos {cosine if cosine is None else round(cosine, 5)} ratio/cos {quotient}"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())

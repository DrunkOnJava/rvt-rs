"""The bar shell's box in the far_vertices frame (site undone, feet), and its bend end in the U's plane.

    python barbox.py <ifc> <Tag>
"""
import os
import sys
import ifcopenshell
import ifcopenshell.geom
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "tools", "re"))
from ifc_common import site_frame  # noqa: E402
from legs import shells  # noqa: E402

path, tag = sys.argv[1], sys.argv[2]
model = ifcopenshell.open(path)
settings = ifcopenshell.geom.settings()
settings.set("use-world-coords", True)
element = next(e for e in model.by_type("IfcProduct") if str(getattr(e, "Tag", "")) == tag and e.Representation)
shape = ifcopenshell.geom.create_shape(settings, element)
verts = site_frame(model)(np.array(shape.geometry.verts, dtype=float).reshape(-1, 3))
faces = np.array(shape.geometry.faces, dtype=int).reshape(-1, 3)
bar = max(shells(verts, faces), key=lambda t: float(np.ptp(verts[faces[t]].reshape(-1, 3)[:, 1])))
pts = np.unique(verts[faces[bar]].reshape(-1, 3), axis=0)
lo, hi = pts.min(axis=0), pts.max(axis=0)
print(f"bar vertices={len(pts)} min={np.round(lo, 4)} max={np.round(hi, 4)}")
mid = (lo[0] + hi[0]) / 2.0
for name, end in (("low-y end", pts[pts[:, 1] < lo[1] + 0.25]), ("high-y end", pts[pts[:, 1] > hi[1] - 0.25])):
    plane = end[np.abs(end[:, 0] - mid) < 0.002]
    tip = plane[np.argmin(plane[:, 1])] if name == "low-y end" else plane[np.argmax(plane[:, 1])]
    print(f"{name}: vertices within 0.25 ft={len(end)}, on the U's plane={len(plane)}, tip y={tip[1]:.4f} at z={tip[2]:.4f}")

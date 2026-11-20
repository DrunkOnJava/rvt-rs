"""One element in both exports: both directions' vertex-to-surface distances and the volume ratio.

    python pair.py <Revit ifc> <rvt-rs ifc> <Tag> ...

Each body in the far_vertices frame (site undone, feet). Prints, per Tag: Revit's vertices'
largest distance from rvt-rs's surface and how many exceed 0.001 / 0.01 ft; the same for rvt-rs's
from Revit's; and rvt-rs's signed volume over Revit's (closed shells assumed; a ratio far from 1
with open shells is not a volume).
"""
import os
import sys
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import functools  # noqa: E402
import place  # noqa: E402
from place import body, distance  # noqa: E402

place.ifcopenshell.open = functools.lru_cache(maxsize=4)(place.ifcopenshell.open)


def far(points, verts, faces):
    a, b, c = verts[faces[:, 0]], verts[faces[:, 1]], verts[faces[:, 2]]
    pts = np.unique(np.round(points, 6), axis=0)
    d = np.array([float(distance(p, a, b, c).min()) for p in pts])
    return d.max(), int((d > 0.001).sum()), int((d > 0.01).sum()), len(pts)


def volume(verts, faces):
    p = verts[faces]
    return float(np.einsum("ij,ij->i", p[:, 0], np.cross(p[:, 1], p[:, 2])).sum() / 6.0)


for tag in sys.argv[3:]:
    try:
        rv, rf = body(sys.argv[1], tag)
        ov, of = body(sys.argv[2], tag)
    except StopIteration:
        print(f"{tag}: not in both"); continue
    r = far(rv[np.unique(rf)], ov, of)
    o = far(ov[np.unique(of)], rv, rf)
    print(f"{tag}: Revit's off ours max {r[0]:.5f} ft (>0.001 {r[1]}, >0.01 {r[2]} of {r[3]}) | "
          f"ours off Revit's max {o[0]:.5f} ft (>0.001 {o[1]}, >0.01 {o[2]} of {o[3]}) | volume ratio {volume(ov, of) / volume(rv, rf):.4f}")

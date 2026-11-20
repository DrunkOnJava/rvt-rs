"""Where the Revit vertices that newly stand off rvt-rs's surface lie (#2107 row 5).

    python row5_where.py <Revit ifc> <base ifc> <head ifc> <Tag>

In place.body's frame (feet). Revit's vertices beyond 0.001 ft of the head's surface but not of
the base's: how far each is from the nearest head vertex that moved from the base, and from the
nearest head vertex that did not; and how far the moved vertices moved.
"""
import functools, sys
import numpy as np
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))
import place
from place import body, distance
place.ifcopenshell.open = functools.lru_cache(maxsize=4)(place.ifcopenshell.open)
revit, base, head, tag = sys.argv[1:5]
rv, rf = body(revit, tag); bv, bf = body(base, tag); hv, hf = body(head, tag)
def dists(points, v, f):
    a, b, c = v[f[:, 0]], v[f[:, 1]], v[f[:, 2]]
    return np.array([float(distance(p, a, b, c).min()) for p in points])
pts = np.unique(np.round(rv[np.unique(rf)], 6), axis=0)
db, dh = dists(pts, bv, bf), dists(pts, hv, hf)
new = pts[(dh > 0.001) & (db <= 0.001)]
print(f"{tag}: Revit vertices {len(pts)}; beyond 0.001 ft base {int((db > 0.001).sum())} head {int((dh > 0.001).sum())}; newly beyond {len(new)}; back within {int(((db > 0.001) & (dh <= 0.001)).sum())}")
def nearest(points, cloud):
    out = np.empty(len(points))
    for i in range(0, len(points), 512):
        chunk = points[i:i + 512]
        out[i:i + 512] = np.sqrt(((chunk[:, None, :] - cloud[None, :, :]) ** 2).sum(-1)).min(1)
    return out
shift = nearest(hv, bv)
moved = hv[shift > 1e-9]; still = hv[shift <= 1e-9]
print(f"  head vertices not at a base vertex {len(moved)} of {len(hv)}, up to {shift.max():.6f} ft from one (median {np.median(shift[shift > 1e-9]) if len(moved) else 0:.6f})")
if len(new) and len(moved):
    to_moved = nearest(new, moved); to_still = nearest(new, still)
    print(f"  newly beyond: nearest moved vertex median {np.median(to_moved):.5f} ft, max {to_moved.max():.5f}; nearest unmoved median {np.median(to_still):.5f} ft")
    near_moved = int((to_moved < to_still).sum())
    print(f"  newly beyond nearer a moved vertex than an unmoved one: {near_moved} of {len(new)}")
    dn = dh[(dh > 0.001) & (db <= 0.001)]
    print(f"  newly beyond, their distance from the head surface: median {np.median(dn):.5f} max {dn.max():.5f} ft")

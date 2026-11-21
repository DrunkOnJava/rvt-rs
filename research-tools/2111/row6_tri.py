"""For a body's Revit vertices newly beyond 0.001 ft of the head's surface: the nearest head triangle
(its corners, which are new, its edge lengths) and the nearest base triangle, in place.body's frame.

    python row6_tri.py <Revit ifc> <base ifc> <head ifc> <Tag> [n]
"""
import functools, os, sys
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import place
from place import body, distance
place.ifcopenshell.open = functools.lru_cache(maxsize=4)(place.ifcopenshell.open)
revit, base, head, tag = sys.argv[1:5]
n = int(sys.argv[5]) if len(sys.argv) > 5 else 3
rv, rf = body(revit, tag); bv, bf = body(base, tag); hv, hf = body(head, tag)
def tri_d(p, v, f):
    a, b, c = v[f[:, 0]], v[f[:, 1]], v[f[:, 2]]
    return np.asarray(distance(p, a, b, c))
pts = np.unique(np.round(rv[np.unique(rf)], 6), axis=0)
new_pts = []
for p in pts:
    db, dh = tri_d(p, bv, bf).min(), tri_d(p, hv, hf).min()
    if dh > 0.001 and db <= 0.001:
        new_pts.append((p, db, dh))
base_set = {tuple(np.round(x, 6)) for x in bv}
print(f"{tag}: newly beyond {len(new_pts)}")
for p, db, dh in new_pts[:n]:
    i = int(np.argmin(tri_d(p, hv, hf))); j = int(np.argmin(tri_d(p, bv, bf)))
    ht = hv[hf[i]]; bt = bv[bf[j]]
    newc = [tuple(np.round(x, 6)) not in base_set for x in ht]
    el = lambda t: [round(float(np.linalg.norm(t[(k + 1) % 3] - t[k])), 5) for k in range(3)]
    print(f"  Revit vertex {np.round(p, 4)}: base {db:.5f} ft, head {dh:.5f} ft")
    print(f"    head triangle corners new {newc}, edges {el(ht)} ft")
    print(f"    base triangle edges {el(bt)} ft")
    # Is the head triangle's plane far from the Revit vertex because it is a sliver across the curve?
    nrm = np.cross(ht[1] - ht[0], ht[2] - ht[0]); area = np.linalg.norm(nrm) / 2
    print(f"    head triangle area {area:.3e} ft2; base triangle area {np.linalg.norm(np.cross(bt[1]-bt[0], bt[2]-bt[0]))/2:.3e}")

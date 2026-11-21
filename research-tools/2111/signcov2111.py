"""#2111: for Revit vertices newly beyond 0.001 ft of head's surface, the sign of the offset from the
nearest head triangle (by its winding, + outward), and whether main's nearest triangle is still
covered by head's mesh (its centroid and edge midpoints within 0.001 ft of head's surface).

    python signcov2111.py <Revit ifc> <base ifc> <head ifc> <Tag> [<Tag> ...]
"""
import functools, os, sys
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import place
from place import body, distance
place.ifcopenshell.open = functools.lru_cache(maxsize=4)(place.ifcopenshell.open)
def tris(v, f): return v[f[:, 0]], v[f[:, 1]], v[f[:, 2]]
revit, base, head = sys.argv[1:4]
for tag in sys.argv[4:]:
    rv, rf = body(revit, tag); bv, bf = body(base, tag); hv, hf = body(head, tag)
    bt, ht = tris(bv, bf), tris(hv, hf)
    centre = hv.mean(axis=0)
    pts = np.unique(np.round(rv[np.unique(rf)], 6), axis=0)
    signs = []; uncovered = 0; checked = 0; worst_uncov = 0.0; outward_normals = 0
    for p in pts:
        db = distance(p, *bt); dh = distance(p, *ht)
        if dh.min() > 0.001 and db.min() <= 0.001:
            i = int(np.argmin(dh)); A, B, C = (x[i] for x in ht)
            n = np.cross(B - A, C - A); n /= np.linalg.norm(n) or 1.0
            if np.dot(n, A - centre) > 0: outward_normals += 1
            signs.append(float(np.dot(p - A, n)))
            j = int(np.argmin(db)); a, b, c = (x[j] for x in bt)
            for q in (a + b + c) / 3, (a + b) / 2, (b + c) / 2, (c + a) / 2:
                checked += 1
                d = distance(q, *ht).min()
                worst_uncov = max(worst_uncov, d)
                if d > 0.001: uncovered += 1
    if signs:
        s = np.array(signs)
        print(f"{tag}: {len(s)} newly beyond; offset by winding: outward {int((s > 0).sum())}, inward {int((s < 0).sum())} (median {np.median(s):+.5f} ft); nearest head triangles whose normal points away from the body's centre {outward_normals} of {len(s)}; main's nearest triangle points checked {checked}, beyond 0.001 ft of head's surface {uncovered}, farthest {worst_uncov:.5f} ft")
    else:
        print(f"{tag}: none newly beyond")

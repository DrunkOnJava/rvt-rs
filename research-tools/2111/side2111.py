"""#2111: for Revit vertices newly beyond 0.001 ft of head's surface, whether the Revit vertex and its
nearest point on base's mesh lie on the same side of the nearest head triangle (no winding needed);
and for points of base's nearest triangle beyond 0.001 ft of head's mesh, whether their nearest head
point is inside a triangle or on an edge, and whether that edge is used once in the welded head mesh.

    python side2111.py <Revit ifc> <base ifc> <head ifc> <Tag> [<Tag> ...]
"""
import functools, os, sys
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import place
from place import body, distance
place.ifcopenshell.open = functools.lru_cache(maxsize=4)(place.ifcopenshell.open)
def tris(v, f): return v[f[:, 0]], v[f[:, 1]], v[f[:, 2]]
def closest_on(p, A, B, C):
    # Ericson's closest point on one triangle, and whether it is interior
    ab, ac, ap = B - A, C - A, p - A
    d1, d2 = ab @ ap, ac @ ap
    if d1 <= 0 and d2 <= 0: return A, "vertex"
    bp = p - B; d3, d4 = ab @ bp, ac @ bp
    if d3 >= 0 and d4 <= d3: return B, "vertex"
    vc = d1 * d4 - d3 * d2
    if vc <= 0 and d1 >= 0 and d3 <= 0: return A + ab * (d1 / (d1 - d3)), "edge"
    cp = p - C; d5, d6 = ab @ cp, ac @ cp
    if d6 >= 0 and d5 <= d6: return C, "vertex"
    vb = d5 * d2 - d1 * d6
    if vb <= 0 and d2 >= 0 and d6 <= 0: return A + ac * (d2 / (d2 - d6)), "edge"
    va = d3 * d6 - d5 * d4
    if va <= 0 and (d4 - d3) >= 0 and (d5 - d6) >= 0:
        return B + (C - B) * ((d4 - d3) / ((d4 - d3) + (d5 - d6))), "edge"
    den = 1 / (va + vb + vc); return A + ab * (vb * den) + ac * (vc * den), "inside"
def nearest(p, v, f):
    t = tris(v, f); d = distance(p, *t); i = int(np.argmin(d))
    q, kind = closest_on(p, t[0][i], t[1][i], t[2][i]); return float(d[i]), q, kind, i
def once_edges(v, f):
    key = [tuple(np.round(x, 6)) for x in v]; uses = {}
    for t in f:
        for x, y in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0])):
            e = tuple(sorted((key[x], key[y]))); uses[e] = uses.get(e, 0) + 1
    return [(np.array(a), np.array(b)) for (a, b), n in uses.items() if n == 1]
def seg(p, a, b):
    ab = b - a; t = np.clip(((p - a) @ ab) / ((ab @ ab) or 1.0), 0, 1); return float(np.linalg.norm(a + t * ab - p))
revit, base, head = sys.argv[1:4]
for tag in sys.argv[4:]:
    rv, rf = body(revit, tag); bv, bf = body(base, tag); hv, hf = body(head, tag)
    pts = np.unique(np.round(rv[np.unique(rf)], 6), axis=0)
    opens = once_edges(hv, hf)
    same = diff = 0; uncov = []
    for p in pts:
        db, qb, _, j = nearest(p, bv, bf); dh, qh, _, i = nearest(p, hv, hf)
        if not (dh > 0.001 and db <= 0.001): continue
        A, B, C = (x[i] for x in tris(hv, hf)); n = np.cross(B - A, C - A); n /= np.linalg.norm(n) or 1.0
        sp, sq = (p - A) @ n, (qb - A) @ n
        if sp * sq > 0: same += 1
        else: diff += 1
        a, b, c = (x[j] for x in tris(bv, bf))
        for q in (a + b + c) / 3, (a + b) / 2, (b + c) / 2, (c + a) / 2:
            d, qq, kind, _ = nearest(q, hv, hf)
            if d > 0.001:
                on_open = min((seg(qq, x, y) for x, y in opens), default=1.0) < 1e-6
                uncov.append((round(d, 5), kind, on_open))
    print(f"{tag}: Revit vertex and main's nearest point on the same side of the nearest new triangle {same}, opposite {diff}; main's points beyond 0.001 ft of the new mesh {len(uncov)}: {uncov}")

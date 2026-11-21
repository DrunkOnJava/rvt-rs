"""#2111: for a body, (1) rvt-rs's farthest vertex from Revit's surface on head, and its
distance from base's own mesh; (2) Revit's vertices newly beyond 0.001 ft of head's surface:
whether the nearest head point is inside a triangle (offset along its normal) or on an edge,
and how many edges of the welded head mesh used once lie within 0.01 ft of it.

    python q2111.py <Revit ifc> <base ifc> <head ifc> <Tag> [<Tag> ...]
"""
import functools, os, sys
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import place
from place import body, distance
place.ifcopenshell.open = functools.lru_cache(maxsize=4)(place.ifcopenshell.open)

def tris(v, f):
    return v[f[:, 0]], v[f[:, 1]], v[f[:, 2]]

def nearest(p, v, f):
    a, b, c = tris(v, f)
    d = distance(p, a, b, c)
    i = int(np.argmin(d))
    A, B, C = a[i], b[i], c[i]
    n = np.cross(B - A, C - A); n = n / (np.linalg.norm(n) or 1.0)
    along = float(np.dot(p - A, n))
    q = p - along * n
    def area(x, y, z): return np.dot(np.cross(y - x, z - x), n)
    tot = area(A, B, C)
    bary = [area(q, B, C) / tot, area(A, q, C) / tot, area(A, B, q) / tot] if tot else [0, 0, 0]
    interior = min(bary) > 1e-6
    return float(d[i]), interior, abs(along), i

def open_edges(v, f):
    key = [tuple(np.round(x, 6)) for x in v]
    uses = {}
    for t in f:
        for x, y in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0])):
            e = tuple(sorted((key[x], key[y])))
            uses[e] = uses.get(e, 0) + 1
    return [e for e, n in uses.items() if n == 1]

def seg_dist(p, a, b):
    a, b = np.array(a), np.array(b); ab = b - a
    t = np.clip(np.dot(p - a, ab) / (np.dot(ab, ab) or 1.0), 0, 1)
    return float(np.linalg.norm(a + t * ab - p))

revit, base, head = sys.argv[1:4]
for tag in sys.argv[4:]:
    rv, rf = body(revit, tag); bv, bf = body(base, tag); hv, hf = body(head, tag)
    ra, rb, rc = tris(rv, rf)
    hd = np.array([distance(p, ra, rb, rc).min() for p in hv])
    k = int(np.argmax(hd)); p = hv[k]
    db, interior_b, along_b, _ = nearest(p, bv, bf)
    print(f"{tag}: head vertices {len(hv)}, base {len(bv)}; head's farthest from Revit {hd[k]:.5f} ft, from base's own mesh {db:.6f} ft ({'inside a triangle' if interior_b else 'on an edge'})")
    bd = np.array([distance(q, ra, rb, rc).min() for q in bv]) if len(bv) else np.array([0.0])
    print(f"   base's farthest from Revit {bd.max():.5f} ft")
    pts = np.unique(np.round(rv[np.unique(rf)], 6), axis=0)
    opens = open_edges(hv, hf)
    rows = []
    for q in pts:
        bb = distance(q, *tris(bv, bf)).min()
        dh, interior, along, _ = nearest(q, hv, hf)
        if dh > 0.001 and bb <= 0.001:
            near_open = sum(1 for e in opens if seg_dist(q, *e) < 0.01)
            rows.append((dh, interior, along, near_open))
    if rows:
        ins = sum(1 for r in rows if r[1]); on = len(rows) - ins
        print(f"   Revit vertices newly beyond 0.001 ft: {len(rows)}; nearest head point inside a triangle {ins} (median normal offset {np.median([r[2] for r in rows if r[1]] or [0]):.5f}), on an edge {on}; with an open head edge within 0.01 ft: {sum(1 for r in rows if r[3])}")
    print(f"   head open edges: {len(opens)}; base open edges: {len(open_edges(bv, bf))}")

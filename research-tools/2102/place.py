"""Where Revit's vertices that lie off rvt-rs's surface are, and on which side of it.

    python place.py <Revit ifc> <rvt-rs ifc> <Tag> [<bar ft>]

Both bodies are meshed in world coordinates and moved into the frame far_vertices_vs_ifc uses
(each file's site undone, feet). For every Revit vertex, the distance to rvt-rs's surface is the
least point-to-triangle distance over rvt-rs's triangles; those beyond the bar (0.001 ft) are
placed: inside or outside rvt-rs's body (ray parity against all its triangles, which counts right
for disjoint closed shells), which of Revit's shells holds them, and where in the body's box
(fractions along x, y, z).
"""
import collections
import os
import sys

import ifcopenshell
import ifcopenshell.geom
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "tools", "re"))
from ifc_common import site_frame  # noqa: E402
from legs import shells  # noqa: E402


def body(path, tag):
    model = ifcopenshell.open(path)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    element = next(e for e in model.by_type("IfcProduct") if str(getattr(e, "Tag", "")) == tag and e.Representation)
    shape = ifcopenshell.geom.create_shape(settings, element)
    verts = site_frame(model)(np.array(shape.geometry.verts, dtype=float).reshape(-1, 3))
    return verts, np.array(shape.geometry.faces, dtype=int).reshape(-1, 3)


def distance(p, a, b, c):
    """Least distance from point p to each triangle (a, b, c), Ericson's closest-point regions."""
    ab, ac, ap = b - a, c - a, p - a
    d1, d2 = np.einsum("ij,ij->i", ab, ap), np.einsum("ij,ij->i", ac, ap)
    bp = p - b
    d3, d4 = np.einsum("ij,ij->i", ab, bp), np.einsum("ij,ij->i", ac, bp)
    cp = p - c
    d5, d6 = np.einsum("ij,ij->i", ab, cp), np.einsum("ij,ij->i", ac, cp)
    va, vb, vc = d3 * d6 - d5 * d4, d5 * d2 - d1 * d6, d1 * d4 - d3 * d2
    denom = va + vb + vc
    with np.errstate(divide="ignore", invalid="ignore"):
        v = np.where(np.abs(denom) > 1e-300, vb / denom, 0.0)
        w = np.where(np.abs(denom) > 1e-300, vc / denom, 0.0)
    q = a + ab * v[:, None] + ac * w[:, None]
    with np.errstate(divide="ignore", invalid="ignore"):
        t_ab = np.clip(np.where(d1 - d3 != 0, d1 / (d1 - d3), 0.0), 0, 1)
        t_ac = np.clip(np.where(d2 - d6 != 0, d2 / (d2 - d6), 0.0), 0, 1)
        t_bc = np.clip(np.where((d4 - d3) + (d5 - d6) != 0, (d4 - d3) / ((d4 - d3) + (d5 - d6)), 0.0), 0, 1)
    edges = np.stack([a + ab * t_ab[:, None], a + ac * t_ac[:, None], b + (c - b) * t_bc[:, None]])
    outside = (va <= 0) | (vb <= 0) | (vc <= 0)
    best_edge = np.min(np.linalg.norm(edges - p, axis=2), axis=0)
    return np.where(outside, best_edge, np.linalg.norm(q - p, axis=1))


def inside(p, a, b, c):
    direction = np.array([0.5773, 0.5774, 0.5775])
    e1, e2 = b - a, c - a
    h = np.cross(direction, e2)
    det = np.einsum("ij,ij->i", e1, h)
    ok = np.abs(det) > 1e-14
    with np.errstate(divide="ignore", invalid="ignore"):
        inv = 1.0 / det
        s = p - a
        u = np.einsum("ij,ij->i", s, h) * inv
        q = np.cross(s, e1)
        v = (q @ direction) * inv
        t = np.einsum("ij,ij->i", e2, q) * inv
    hits = ok & (u >= 0) & (v >= 0) & (u + v <= 1) & (t > 0)
    return int(hits.sum()) % 2 == 1


def main():
    revit_path, ours_path, tag = sys.argv[1], sys.argv[2], sys.argv[3]
    bar = float(sys.argv[4]) if len(sys.argv) > 4 else 0.001
    rv, rf = body(revit_path, tag)
    ov, of = body(ours_path, tag)
    a, b, c = ov[of[:, 0]], ov[of[:, 1]], ov[of[:, 2]]
    ours_shell = np.full(len(of), -1)
    closed = {}
    for k, tris in enumerate(sorted(shells(ov, of), key=len, reverse=True)):
        ours_shell[tris] = k
        key = {}
        canon = [key.setdefault(tuple(np.round(v, 6)), j) for j, v in enumerate(ov)]
        edges = collections.Counter()
        for t in tris:
            x, y, z = (canon[j] for j in of[t])
            for e in ((x, y), (y, z), (z, x)):
                edges[frozenset(e)] += 1
        closed[k] = all(n == 2 for n in edges.values())
    owner = np.full(len(rv), -1)
    for k, tris in enumerate(sorted(shells(rv, rf), key=len, reverse=True)):
        owner[rf[tris].ravel()] = k
    used = np.unique(rf.ravel())
    lo, hi = np.minimum(rv.min(axis=0), ov.min(axis=0)), np.maximum(rv.max(axis=0), ov.max(axis=0))
    counts = collections.Counter()
    worst = []
    seen = set()
    for i in used:
        key = tuple(np.round(rv[i], 6))
        if key in seen:
            continue
        seen.add(key)
        dist = distance(rv[i], a, b, c)
        d = float(dist.min())
        if d <= bar:
            continue
        near = int(ours_shell[int(np.argmin(dist))])
        pick = ours_shell == near
        side = ("inside" if inside(rv[i], a[pick], b[pick], c[pick]) else "outside") if closed[near] else f"open{near}"
        frac = tuple(round(float(x), 2) for x in (rv[i] - lo) / np.where(hi > lo, hi - lo, 1))
        counts[(side, int(owner[i]))] += 1
        worst.append((d, side, int(owner[i]), frac, tuple(round(float(x), 4) for x in rv[i])))
    print(f"{tag}: Revit vertices {len(seen)}; beyond {bar} ft of rvt-rs's surface {sum(counts.values())}")
    for (side, shell), n in sorted(counts.items()):
        ds = [w[0] for w in worst if w[1] == side and w[2] == shell]
        print(f"  {side:7} Revit shell {shell}: {n:4}  largest {max(ds):.5f} ft")
    for d, side, shell, frac, xyz in sorted(worst, reverse=True)[:12]:
        print(f"    {d:.5f} ft {side} shell {shell} at {frac}")
    for shell in sorted({w[2] for w in worst}):
        ys = collections.Counter(round(w[3][1], 1) for w in worst if w[2] == shell)
        print(f"  Revit shell {shell} by y fraction (0.1 bins):", dict(sorted(ys.items())))
    show = os.environ.get("SHOW_SHELL")
    if show is not None:
        for d, side, shell, frac, xyz in sorted(w for w in worst if w[2] == int(show)):
            print(f"    shell {shell} {side:7} {d:.5f} ft at {frac} xyz {xyz}")


if __name__ == "__main__":
    main()

"""Cross-sections of a bent bar, cut square to its longest straight leg.

    python slice.py <ifc> <Tag> [<fraction along the leg> ...]

The body's longest shell (the bar) is read in its own frame. The leg is the direction of the
most triangles whose longest edge exceeds 0.2 (the file's unit). The shell is cut by planes
square to it at the given fractions of the shell's extent along it (0.3, 0.5, 0.7 by default).
Each cut's points are grouped into loops by distance (within a third of the thinnest extent),
and for each loop it prints the number of points, the loop's area (points ordered by angle about
their centroid) and the points' distances from that centroid. A regular n-sided section gives
n points (or 2n where each side is split into two triangles along the cut), all at one distance.
"""
import collections
import os
import sys

import ifcopenshell
import ifcopenshell.geom
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from legs import shells  # noqa: E402


def main():
    path, tag = sys.argv[1], sys.argv[2]
    fractions = [float(x) for x in sys.argv[3:]] or [0.3, 0.5, 0.7]
    model = ifcopenshell.open(path)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    element = next(e for e in model.by_type("IfcProduct") if str(getattr(e, "Tag", "")) == tag and e.Representation)
    shape = ifcopenshell.geom.create_shape(settings, element)
    verts = np.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
    faces = np.array(shape.geometry.faces, dtype=int).reshape(-1, 3)
    best = None
    for tris in shells(verts, faces):
        pts = verts[faces[tris]].reshape(-1, 3)
        span = float(np.ptp(pts - pts.mean(axis=0), axis=0).max())
        if best is None or span > best[1]:
            best = (tris, span)
    tris = best[0]
    tri = verts[faces[tris]]
    votes = []
    for p in tri:
        edges = [p[1] - p[0], p[2] - p[1], p[0] - p[2]]
        lengths = [float(np.linalg.norm(e)) for e in edges]
        k = int(np.argmax(lengths))
        if lengths[k] >= 0.2:
            d = edges[k] / lengths[k]
            for v in votes:
                if abs(float(np.dot(v[0], d))) >= np.cos(np.radians(2.0)):
                    v[1] += 1
                    break
            else:
                votes.append([d, 1])
    d = max(votes, key=lambda v: v[1])[0]
    pts = tri.reshape(-1, 3)
    along = pts @ d
    _, _, axes = np.linalg.svd(pts - pts.mean(axis=0), full_matrices=False)
    thin = float(np.ptp((pts - pts.mean(axis=0)) @ axes[2]))
    e1 = np.cross(d, axes[2] if abs(float(np.dot(d, axes[2]))) < 0.9 else axes[1])
    e1 /= np.linalg.norm(e1)
    e2 = np.cross(d, e1)
    print(f"{tag}: bar triangles={len(tris)} leg votes={max(v[1] for v in votes)} along {along.min():.4f}..{along.max():.4f}")
    for frac in fractions:
        level = along.min() + frac * (along.max() - along.min())
        cut = []
        for p in tri:
            s = p @ d - level
            if s.min() > 0 or s.max() < 0:
                continue
            for i, j in ((0, 1), (1, 2), (2, 0)):
                if (s[i] < 0) != (s[j] < 0):
                    t = s[i] / (s[i] - s[j])
                    q = p[i] + t * (p[j] - p[i])
                    cut.append([float(q @ e1), float(q @ e2)])
        if not cut:
            print(f"  cut at {frac}: nothing")
            continue
        cut = np.unique(np.round(np.array(cut), 7), axis=0)
        label = list(range(len(cut)))
        for i in range(len(cut)):
            for j in range(i):
                if np.linalg.norm(cut[i] - cut[j]) <= 0.03:
                    a, b = label[i], label[j]
                    label = [b if x == a else x for x in label]
        loops = collections.defaultdict(list)
        for i, lab in enumerate(label):
            loops[lab].append(cut[i])
        for loop in sorted(loops.values(), key=lambda l: np.mean(l, axis=0)[0]):
            loop = np.array(loop)
            c = loop.mean(axis=0)
            ang = np.arctan2(loop[:, 1] - c[1], loop[:, 0] - c[0])
            ring = loop[np.argsort(ang)]
            x, y = ring[:, 0], ring[:, 1]
            area = 0.5 * abs(float(np.dot(x, np.roll(y, -1)) - np.dot(y, np.roll(x, -1))))
            cx = float(np.dot(x + np.roll(x, -1), x * np.roll(y, -1) - np.roll(x, -1) * y)) / (6.0 * area) if area > 0 else c[0]
            cy = float(np.dot(y + np.roll(y, -1), x * np.roll(y, -1) - np.roll(x, -1) * y)) / (6.0 * area) if area > 0 else c[1]
            corners = []
            n = len(ring)
            for k in range(n):
                a, b, q = ring[k - 1], ring[k], ring[(k + 1) % n]
                u, v = b - a, q - b
                if abs(u[0] * v[1] - u[1] * v[0]) > 1e-3 * np.linalg.norm(u) * np.linalg.norm(v):
                    corners.append(b)
            corners = np.array(corners) if corners else ring
            rho = np.linalg.norm(corners - np.array([abs(cx) * np.sign(cx), cy]), axis=1)
            print(f"  cut at {frac}: points={len(loop):3} corners={len(corners):3} area={area:.7f} "
                  f"corner radius min={rho.min():.5f} median={np.median(rho):.5f} max={rho.max():.5f}")


main()

"""Per connected shell of one element's body: open edges and edges wound the same way twice.

    python edges.py <ifc> <Tag>

Vertices are welded at a millionth of the file's unit. An open edge has one triangle; a
same-way edge is run in one direction by two triangles (the two faces on it wound alike, so
the shell is not wound one way round); an edge of three or more triangles is counted apart.
"""
import collections
import sys

import ifcopenshell
import ifcopenshell.geom
import numpy as np


def main():
    path, tag = sys.argv[1], sys.argv[2]
    model = ifcopenshell.open(path)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    element = next(
        e for e in model.by_type("IfcProduct") if str(getattr(e, "Tag", "")) == tag and e.Representation
    )
    shape = ifcopenshell.geom.create_shape(settings, element)
    verts = np.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
    faces = np.array(shape.geometry.faces, dtype=int).reshape(-1, 3)
    key = {}
    canon = [key.setdefault(tuple(np.round(v, 6)), i) for i, v in enumerate(verts)]
    tris = [tuple(canon[i] for i in f) for f in faces]
    parent = {}

    def find(i):
        parent.setdefault(i, i)
        while parent[i] != i:
            parent[i] = parent[parent[i]]
            i = parent[i]
        return i

    for a, b, c in tris:
        for x, y in ((a, b), (b, c)):
            ra, rb = find(x), find(y)
            if ra != rb:
                parent[ra] = rb
    shells = collections.defaultdict(list)
    for t in tris:
        shells[find(t[0])].append(t)
    out = []
    for group in shells.values():
        directed = collections.Counter()
        undirected = collections.Counter()
        for a, b, c in group:
            if len({a, b, c}) < 3:
                continue
            for x, y in ((a, b), (b, c), (c, a)):
                directed[(x, y)] += 1
                undirected[frozenset((x, y))] += 1
        opened = sum(1 for n in undirected.values() if n == 1)
        many = sum(1 for n in undirected.values() if n > 2)
        same = sum(1 for n in directed.values() if n > 1)
        p = verts[np.array([list(t) for t in group])]
        vol = float(np.einsum("ij,ij->i", p[:, 0], np.cross(p[:, 1], p[:, 2])).sum() / 6.0)
        area = float(np.linalg.norm(np.cross(p[:, 1] - p[:, 0], p[:, 2] - p[:, 0]), axis=1).sum() / 2.0)
        pts = p.reshape(-1, 3)
        centred = pts - pts.mean(axis=0)
        _, _, axes = np.linalg.svd(centred, full_matrices=False)
        extents = sorted((float(np.ptp(centred @ ax)) for ax in axes), reverse=True)
        out.append((vol, len(group), opened, same, many, area, extents))
    for vol, n, opened, same, many, area, ext in sorted(out, key=lambda s: -abs(s[0])):
        print(f"shell vol={vol:.6f} area={area:.5f} extents={[round(e, 4) for e in ext]} tris={n} open_edges={opened} same_way_edges={same}")


main()

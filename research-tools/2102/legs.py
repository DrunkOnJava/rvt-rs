"""A bent bar's straight legs: for each, its sides and its corner radius, one leg at a time.

    python legs.py <ifc> <Tag> [<min edge length>]

Only the body's longest shell (the bar) is read. Its triangles whose longest edge is over the
given length (0.2 by default, in the file's unit) are grouped by that edge's direction (within
2 degrees, either sense), then split by position across it: triangles whose centroids lie within
a fifth of the shell's thinnest extent of each other, measured square to the direction, are one
leg. For each leg: triangles, length, the distinct normals (within 5 degrees), which are the
section's sides, and the vertices' distances from the line through their mean along the leg.
For a prism of n equal sides those distances are all one corner radius.
"""
import collections
import sys

import ifcopenshell
import ifcopenshell.geom
import numpy as np


def shells(verts, faces):
    key = {}
    canon = [key.setdefault(tuple(np.round(v, 6)), i) for i, v in enumerate(verts)]
    parent = {}

    def find(i):
        parent.setdefault(i, i)
        while parent[i] != i:
            parent[i] = parent[parent[i]]
            i = parent[i]
        return i

    for a, b, c in faces:
        for x, y in ((canon[a], canon[b]), (canon[b], canon[c])):
            ra, rb = find(x), find(y)
            if ra != rb:
                parent[ra] = rb
    groups = collections.defaultdict(list)
    for t, f in enumerate(faces):
        groups[find(canon[f[0]])].append(t)
    return list(groups.values())


def main():
    path, tag = sys.argv[1], sys.argv[2]
    least = float(sys.argv[3]) if len(sys.argv) > 3 else 0.2
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
        centred = pts - pts.mean(axis=0)
        _, _, axes = np.linalg.svd(centred, full_matrices=False)
        extents = sorted((float(np.ptp(centred @ ax)) for ax in axes), reverse=True)
        if best is None or extents[0] > best[1][0]:
            best = (tris, extents)
    tris, extents = best
    print(f"{tag}: bar shell triangles={len(tris)} extents={[round(e, 4) for e in extents]}")
    groups = []
    for t in tris:
        p = verts[faces[t]]
        edges = [p[1] - p[0], p[2] - p[1], p[0] - p[2]]
        lengths = [float(np.linalg.norm(e)) for e in edges]
        k = int(np.argmax(lengths))
        if lengths[k] < least:
            continue
        d = edges[k] / lengths[k]
        for g in groups:
            if abs(float(np.dot(g[0], d))) >= np.cos(np.radians(2.0)):
                g[1].append(t)
                break
        else:
            groups.append([d, [t]])
    reach = extents[2] / 5.0
    legs = []
    for d, members in groups:
        cents = verts[faces[members]].mean(axis=1)
        across = cents - np.outer(cents @ d, d)
        label = list(range(len(members)))
        for i in range(len(members)):
            for j in range(i):
                if np.linalg.norm(across[i] - across[j]) <= reach:
                    a, b = label[i], label[j]
                    label = [b if x == a else x for x in label]
        split = collections.defaultdict(list)
        for i, lab in enumerate(label):
            split[lab].append(members[i])
        legs.extend((d, part) for part in split.values())
    for d, part in sorted(legs, key=lambda g: -len(g[1])):
        p = verts[faces[part]]
        pts = p.reshape(-1, 3)
        rel = pts - pts.mean(axis=0)
        along = rel @ d
        rho = np.linalg.norm(rel - np.outer(along, d), axis=1)
        normals = np.cross(p[:, 1] - p[:, 0], p[:, 2] - p[:, 0])
        normals /= np.linalg.norm(normals, axis=1)[:, None]
        sides = []
        for n in normals:
            if not any(float(np.dot(n, s)) >= np.cos(np.radians(5.0)) for s in sides):
                sides.append(n)
        print(f"  leg triangles={len(part):4} length={np.ptp(along):.4f} sides={len(sides):3} "
              f"radius min={rho.min():.5f} median={np.median(rho):.5f} max={rho.max():.5f}")


if __name__ == "__main__":
    main()

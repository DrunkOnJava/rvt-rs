#!/usr/bin/env python3
"""Cracks inside the cells of one face of a saved mesh (#1737, RE-437).

A face of a body drawn from its saved graphics is one connected piece of an
`IfcTriangulatedFaceSet` (faces do not share vertices). A vertex of such a piece
that lies strictly inside a boundary edge of the same piece (an edge one triangle
uses) is a T-junction: the edge of one cell runs straight past a vertex of the
next, and the two leave a crack as wide as the surface bows over the edge. The
count is of vertices within 0.003 ft of such an edge and clear of its ends
(0.02 to 0.98 of its length), per body class. A change that cuts a face into
fewer cells is held to the count of the build before it: the criterion is
rvtDevelop's, from the read of #1769.

Usage: mesh_tjunctions.py MODEL.IFC [OUT.json]
Prints the totals and, per class of the owning product, the vertices counted.
"""
import collections
import json
import sys

import ifcopenshell
import ifcopenshell.util.unit as unit
import numpy as np

TOLERANCE_FEET = 0.003


def owner(model, face_set):
    """The GlobalId and class of the product whose body holds `face_set`."""
    for representation in model.get_inverse(face_set):
        for definition in model.get_inverse(representation):
            for product in model.get_inverse(definition):
                if hasattr(product, "GlobalId"):
                    return product.GlobalId, product.is_a()
    return None, None


def components(triangles, count):
    """The connected piece of every vertex: vertices that share a triangle are one."""
    parent = list(range(count))

    def find(x):
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    for a, b, c in triangles:
        root = find(a)
        parent[find(b)] = root
        parent[find(c)] = root
    return [find(i) for i in range(count)]


def cracks(vertices, triangles, tolerance):
    """The vertices that lie inside a boundary edge of their own piece."""
    piece = components(triangles, len(vertices))
    uses = collections.Counter()
    for a, b, c in triangles:
        for p, q in ((a, b), (b, c), (c, a)):
            uses[(min(p, q), max(p, q))] += 1
    boundary = collections.defaultdict(list)
    for edge, count in uses.items():
        if count == 1:
            boundary[piece[edge[0]]].append(edge)
    found = 0
    for edges in boundary.values():
        if len(edges) < 3:
            continue
        a = vertices[[e[0] for e in edges]]
        b = vertices[[e[1] for e in edges]]
        verts = sorted({v for e in edges for v in e})
        points = vertices[verts]
        ab = b - a
        length2 = (ab * ab).sum(axis=1)
        length2[length2 == 0] = 1e-300
        t = ((points[:, None, :] - a[None, :, :]) * ab[None, :, :]).sum(axis=2) / length2[None, :]
        foot = a[None, :, :] + t[:, :, None] * ab[None, :, :]
        distance = np.linalg.norm(points[:, None, :] - foot, axis=2)
        inside = (t > 0.02) & (t < 0.98) & (distance < tolerance) & (distance > 1e-12)
        index = {v: i for i, v in enumerate(verts)}
        for j, e in enumerate(edges):
            inside[index[e[0]], j] = False
            inside[index[e[1]], j] = False
        found += int(inside.any(axis=1).sum())
    return found


def main():
    path = sys.argv[1]
    model = ifcopenshell.open(path)
    # The model's length unit in metres, so that the tolerance is feet in any of them.
    tolerance = TOLERANCE_FEET * 0.3048 / unit.calculate_unit_scale(model)
    result = {}
    for face_set in model.by_type("IfcTriangulatedFaceSet"):
        vertices = np.array(face_set.Coordinates.CoordList, dtype=float)
        triangles = np.array(face_set.CoordIndex, dtype=int) - 1
        if len(triangles) == 0:
            continue
        guid, cls = owner(model, face_set)
        result[f"{guid}:{face_set.id()}"] = {
            "class": cls,
            "triangles": int(len(triangles)),
            "cracks": cracks(vertices, triangles, tolerance),
        }
    if len(sys.argv) > 2:
        json.dump(result, open(sys.argv[2], "w"))
    total = collections.Counter()
    by_class = collections.Counter()
    for r in result.values():
        total["face sets"] += 1
        total["triangles"] += r["triangles"]
        total["cracks"] += r["cracks"]
        if r["cracks"]:
            total["face sets with a crack"] += 1
            by_class[r["class"]] += r["cracks"]
    print(dict(total))
    print(dict(by_class))


if __name__ == "__main__":
    main()

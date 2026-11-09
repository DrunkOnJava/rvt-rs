#!/usr/bin/env python3
"""Whether two IFC exports of one file draw the same surfaces (#1553).

    python3 tools/re/ifc_surfaces_same.py BASE.ifc HEAD.ifc

For a change that can only reorder or re-split the triangles of a surface (a
newer triangulator, `earcut` 0.4.12 against 0.4.11): the two exports are
paired entity by entity, by STEP id, which two exports of one file by builds
that differ in nothing but a triangulator keep equal (the script checks that
every id has the same entity type in both, and stops if one does not). Each
`IfcTriangulatedFaceSet` and `IfcFacetedBrep` is reduced to its polygons, each
a tuple of its corner points rounded to 1e-6 (a micrometre in the file's
unit), and the two sides' polygons are compared:

  same polygons         the two sides hold the same multiset of polygons, in
                        any order and from any starting corner.
  same surface          not the same polygons, but the same points, and in
                        every plane the same total area (to 1e-9, relative)
                        and the same boundary (the edges an odd number of its
                        polygons hold: a diagonal is held by two triangles
                        and drops out), so the faces are the same and only the
                        diagonals that split them differ.
  different             anything else. The script exits 1 if there is one.

It prints how many surfaces it compared in each class, and the entity ids and
types of the different ones. It reads no Revit export and writes nothing.
"""

import collections
import sys

import ifcopenshell

ROUND = 6
AREA_RELATIVE = 1e-9


def corner(point):
    return tuple(round(c, ROUND) + 0.0 for c in point)


def polygons_of(entity):
    """The polygons of one surface entity, each a tuple of rounded points."""
    if entity.is_a("IfcTriangulatedFaceSet"):
        points = [corner(p) for p in entity.Coordinates.CoordList]
        return [tuple(points[i - 1] for i in triple) for triple in entity.CoordIndex]
    polygons = []
    for face in entity.Outer.CfsFaces:
        for bound in face.Bounds:
            polygons.append(tuple(corner(p.Coordinates) for p in bound.Bound.Polygon))
    return polygons


def canonical(polygon):
    """The polygon from its smallest corner, in whichever direction gives the smaller sequence."""
    n = len(polygon)
    start = min(range(n), key=lambda i: polygon[i])
    forward = polygon[start:] + polygon[:start]
    backward = tuple(reversed(forward))
    backward = backward[-1:] + backward[:-1]
    return min(forward, backward)


def plane_and_area(polygon):
    """The plane (unit normal, signed offset, rounded, sign fixed) and the area of a polygon."""
    nx = ny = nz = 0.0
    for i, (x1, y1, z1) in enumerate(polygon):
        x2, y2, z2 = polygon[(i + 1) % len(polygon)]
        nx += (y1 - y2) * (z1 + z2)
        ny += (z1 - z2) * (x1 + x2)
        nz += (x1 - x2) * (y1 + y2)
    length = (nx * nx + ny * ny + nz * nz) ** 0.5
    if length == 0.0:
        return None, 0.0
    ux, uy, uz = nx / length, ny / length, nz / length
    offset = ux * polygon[0][0] + uy * polygon[0][1] + uz * polygon[0][2]
    if (ux, uy, uz) < (0.0, 0.0, 0.0) and not (ux == uy == uz == 0.0):
        ux, uy, uz, offset = -ux, -uy, -uz, -offset
    key = (round(ux, 5) + 0.0, round(uy, 5) + 0.0, round(uz, 5) + 0.0, round(offset, 5) + 0.0)
    return key, length / 2.0


def plane_totals(polygons):
    """Per plane: the total area, and the edges an odd number of polygons hold."""
    areas = collections.defaultdict(float)
    parity = collections.defaultdict(set)
    for polygon in polygons:
        key, area = plane_and_area(polygon)
        areas[key] += area
        for i, point in enumerate(polygon):
            edge = tuple(sorted((point, polygon[(i + 1) % len(polygon)])))
            parity[key] ^= {edge}
    return areas, parity


def compare(base, head):
    """'same polygons', 'same surface' or 'different'."""
    left = collections.Counter(canonical(p) for p in base)
    right = collections.Counter(canonical(p) for p in head)
    if left == right:
        return "same polygons"
    if {pt for p in base for pt in p} != {pt for p in head for pt in p}:
        return "different"
    (a, edges_a), (b, edges_b) = plane_totals(base), plane_totals(head)
    if a.keys() != b.keys() or edges_a != edges_b:
        return "different"
    for key, area in a.items():
        if abs(area - b[key]) > AREA_RELATIVE * max(1.0, area, b[key]):
            return "different"
    return "same surface"


def main(argv):
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    base, head = ifcopenshell.open(argv[1]), ifcopenshell.open(argv[2])
    kinds = ("IfcTriangulatedFaceSet", "IfcFacetedBrep")
    ids = {kind: sorted(e.id() for e in base.by_type(kind)) for kind in kinds}
    ids_head = {kind: sorted(e.id() for e in head.by_type(kind)) for kind in kinds}
    if ids != ids_head:
        print(f"the two exports hold different surface entities: {({k: len(v) for k, v in ids.items()})} against {({k: len(v) for k, v in ids_head.items()})}")
        return 1
    counts = collections.Counter()
    different = []
    for kind in kinds:
        for step_id in ids[kind]:
            left, right = base.by_id(step_id), head.by_id(step_id)
            verdict = compare(polygons_of(left), polygons_of(right))
            counts[(kind, verdict)] += 1
            if verdict == "different":
                different.append((step_id, kind))
    for (kind, verdict), n in sorted(counts.items()):
        print(f"{kind}: {verdict} {n}")
    print(f"surfaces compared: {sum(counts.values())}; different: {len(different)}")
    for step_id, kind in different[:40]:
        print(f"different: #{step_id} {kind}")
    return 1 if different else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

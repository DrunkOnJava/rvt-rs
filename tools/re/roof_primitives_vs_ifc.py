#!/usr/bin/env python3
"""Set each primitive of one element's saved graphics against Revit's body for it (#1656).

`probe_1656_roof_primitives` prints the primitives of one element's saved graphics, one JSON line
each: a face tag, a style, a material, an owner, and the vertices and triangles in internal feet.
Revit's export holds the same element's body under its Tag. This says, primitive by primitive, how
far each vertex of rvt-rs's lies from Revit's surface, which primitives hold a vertex that is off
it, and how much of the mesh's volume each primitive's faces hold (the volume of the cone from the
mesh's centre over them), so that a volume a little over Revit's can be put on the primitives that
Revit's body does not hold, or on a tessellation that differs. Revit's repeated triangles (a body it
writes with every face twice) count once, as `volume_readings.py` counts them.

Usage:

    python3 tools/re/roof_primitives_vs_ifc.py <probe output> <revit-export.ifc>

Needs IfcOpenShell and numpy, as `mesh_bodies_vs_ifc.py` does.
"""

import json
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mesh_bodies_vs_ifc as meshes  # noqa: E402

# A vertex this close to the other body's surface lies on it (ft).
NEAR = 1e-3
# Vertices listed per primitive and in all.
LISTED = 8


def read_probe(path):
    """The element's Tag, the probe's header line and [(index, primitive)]."""
    tag, header, prims = None, "", []
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            parts = line.rstrip("\n").split("\t", 2)
            if parts[0] == "element" and len(parts) >= 2:
                tag, header = parts[1], line.strip()
            elif parts[0] == "prim" and len(parts) == 3:
                prims.append((int(parts[1]), json.loads(parts[2])))
    return tag, header, prims


def distinct(triangles):
    """`triangles` (n, 3, 3) with each one that repeats an earlier one (the same vertices to 1e-6
    ft, in the same cyclic order) left out."""
    seen, keep = set(), []
    for i, triangle in enumerate(triangles):
        key = tuple(tuple(int(x) for x in v) for v in np.round(triangle * 1e6))
        canonical = min(key[k:] + key[:k] for k in range(3))
        if canonical not in seen:
            seen.add(canonical)
            keep.append(i)
    return triangles[keep]


def shell_groups(triangles):
    """The triangles of `triangles` (n, 3, 3) as lists of indices, one per set joined through shared
    vertices (to 1e-6 ft), the largest first."""
    keys = [tuple(int(x) for x in v) for v in np.round(triangles.reshape(-1, 3) * 1e6)]
    parent = {}

    def find(k):
        while parent.setdefault(k, k) != k:
            parent[k] = parent[parent[k]]
            k = parent[k]
        return k

    for i in range(0, len(keys), 3):
        root = find(keys[i])
        for k in keys[i + 1 : i + 3]:
            parent[find(k)] = root
    groups = {}
    for i in range(len(triangles)):
        groups.setdefault(find(keys[3 * i]), []).append(i)
    return sorted(groups.values(), key=lambda g: -len(g))


def area_and_normal(triangles):
    """The area of `triangles` and their area-weighted mean unit normal."""
    cross = np.cross(triangles[:, 1] - triangles[:, 0], triangles[:, 2] - triangles[:, 0])
    total = cross.sum(axis=0)
    return float(np.linalg.norm(cross, axis=1).sum() / 2), total / max(float(np.linalg.norm(total)), 1e-24)


def cone_volume(triangles, centre):
    """The signed volume of the cones from `centre` over `triangles`."""
    a, b, c = triangles[:, 0] - centre, triangles[:, 1] - centre, triangles[:, 2] - centre
    return float(np.einsum("ij,ij->i", a, np.cross(b, c)).sum() / 6.0)


def vertex_distances(points, triangles):
    """The distance from each of `points` (n, 3) to the nearest of `triangles` (m, 3, 3)."""
    return meshes.distances(points[:, None, :], meshes.triangle_terms(triangles)).min(axis=1)


def compare(prims, revit):
    """The report lines for rvt-rs's primitives (`read_probe`'s) against Revit's triangles (m, 3, 3)."""
    ours = []
    for index, primitive in prims:
        if not primitive["triangles"]:
            continue
        vertices = np.array(primitive["vertices"], dtype=float)
        ours.append((index, primitive, vertices, vertices[np.array(primitive["triangles"], dtype=int)]))
    if not ours:
        return ["rvt-rs's saved graphics hold no triangle"]
    every = np.concatenate([triangles for _, _, _, triangles in ours])
    centre = every.reshape(-1, 3).mean(axis=0)
    once = distinct(revit)
    shells = meshes.shell_volumes(once)
    lines = [
        f"rvt-rs: {len(ours)} primitives, {len(every)} triangles, volume {meshes.volume(every):.6f} ft3",
        f"Revit: {len(revit)} triangles, {len(once)} distinct, {len(shells)} shells of volume"
        f" {', '.join(f'{abs(s):.6f}' for s in shells)} ft3 (distinct triangles), volume {abs(sum(shells)):.6f} ft3",
    ]
    # Revit's shells and what lies between them: a body of several solids that meet at a seam is
    # one solid in a saved mesh, and the seam's width is the volume the mesh holds over Revit's.
    groups = shell_groups(once)
    pieces = [once[g] for g in groups]
    for k, piece in enumerate(pieces):
        low, high = piece.reshape(-1, 3).min(axis=0), piece.reshape(-1, 3).max(axis=0)
        lines.append(
            f"Revit shell {k}: {len(piece)} triangles, volume {abs(cone_volume(piece, piece.reshape(-1, 3).mean(axis=0))):.6f} ft3,"
            f" box ({low[0]:.4f}, {low[1]:.4f}, {low[2]:.4f}) to ({high[0]:.4f}, {high[1]:.4f}, {high[2]:.4f}) ft"
        )
    for a in range(len(pieces)):
        for b in range(a + 1, len(pieces)):
            a_to_b = vertex_distances(pieces[a].reshape(-1, 3), pieces[b])
            b_to_a = vertex_distances(pieces[b].reshape(-1, 3), pieces[a])
            gap = float(min(a_to_b.min(), b_to_a.min()))
            lines.append(f"shells {a} and {b}: the nearest vertex of one is {gap:.6f} ft from the surface of the other")
            for name, mine, other in ((a, pieces[a], pieces[b]), (b, pieces[b], pieces[a])):
                centres = mine.mean(axis=1)
                near = vertex_distances(centres, other) <= gap + 1e-3
                if near.any():
                    area, normal = area_and_normal(mine[near])
                    lines.append(
                        f"  shell {name}: {int(near.sum())} triangles within {gap + 1e-3:.4f} ft of the other, area {area:.4f} ft2,"
                        f" mean normal ({normal[0]:.4f}, {normal[1]:.4f}, {normal[2]:.4f}), area x gap {area * gap:.6f} ft3"
                    )
    theirs = vertex_distances(once.reshape(-1, 3), every)
    lines.append(
        f"Revit's vertices from rvt-rs's surface: farthest {theirs.max():.6f} ft,"
        f" {int((theirs > NEAR).sum())} of {len(theirs)} farther than {NEAR} ft"
    )
    own = {}
    for index, primitive, vertices, triangles in ours:
        own[index] = vertex_distances(vertices, once)
    far = np.concatenate(list(own.values()))
    lines.append(
        f"rvt-rs's vertices from Revit's surface: farthest {far.max():.6f} ft,"
        f" {int((far > NEAR).sum())} of {len(far)} farther than {NEAR} ft"
    )
    listed = []
    for index, primitive, vertices, triangles in ours:
        distance = own[index]
        off = distance > NEAR
        lines.append(
            f"prim {index}: face_tag {primitive['face_tag']} style {primitive['render_style_id']}"
            f" material {primitive['material_id']} owner {primitive['source_owner_id']}"
            f" object {primitive['object_index']}: {len(triangles)} triangles, {len(vertices)} vertices,"
            f" cone volume {cone_volume(triangles, centre):+.6f} ft3,"
            f" vertices off Revit's surface {int(off.sum())} of {len(vertices)},"
            f" farthest {distance.max():.6f} ft"
        )
        for k in np.argsort(-distance)[:LISTED]:
            if off[k]:
                listed.append((distance[k], index, vertices[k]))
    for distance, index, vertex in sorted(listed, key=lambda row: -row[0])[: 2 * LISTED]:
        lines.append(
            f"off-surface vertex of prim {index}: ({vertex[0]:.4f}, {vertex[1]:.4f}, {vertex[2]:.4f}) ft,"
            f" {distance:.6f} ft from Revit's surface"
        )
    return lines


def main():
    # Measure adds `--list` to every probe scorer's arguments; this one has no use for it.
    args = [arg for arg in sys.argv[1:] if not arg.startswith("--")]
    if len(args) != 2:
        print(__doc__)
        return 2
    probe_path, ref_path = args
    tag, header, prims = read_probe(probe_path)
    if tag is None:
        print("the probe printed no element: the model holds no saved graphics under that ElementId")
        return 0
    print(header)
    ref = meshes.meshes(ref_path, tags={tag}, role="ref")
    if tag not in ref:
        print(f"Revit's export holds no body under Tag {tag}")
        return 0
    for line in compare(prims, ref[tag][2]):
        print(line)
    return 0


if __name__ == "__main__":
    sys.exit(main())

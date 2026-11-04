#!/usr/bin/env python3
"""Score each decoded face of a toilet against Revit's mesh of the same toilet
(RE-302, #1247).

`mesh_bodies_vs_ifc.py` judges a whole body: the BIM4LCA office's 43
`WC-istuin:WC-1` toilets, drawn from their saved faces by the second attempt
(Measure run 41766414367, b5d5546f), are 1.0060 of Revit's volume, with our
vertices 0.0215 to 0.0222 ft from Revit's surface and Revit's 0.0046 to
0.0050 ft from ours. This reads `probe_re302_toilet_faces`'s output (every
decoded face of a few toilets, in document feet) and Revit's export, and says,
per face of ours:

- how far its triangle centres, area-weighted, lie from Revit's surface: the
  area beyond 0.001, 0.005 and 0.01 ft, and the largest distance (surface rvt-rs
  draws that Revit's export does not);
- how many of Revit's vertices have this face nearest, and how many of those
  lie more than 0.001 ft from it (surface Revit's export has that this face
  does not reach).

Then the same sums per surface class and, over all of Revit's vertices, the
distribution of their distance to our surface. A face drawn where Revit's
export has nothing shows as area far from Revit's surface; Revit's export
being the coarser polygonisation of the same surface shows as small distances
on both sides and Revit's vertices near our faces.

The per-face table and the sums are printed for every toilet. The list of
Revit's vertices more than 0.001 ft off our surface, the count of our
triangles more than 0.01 ft from Revit's and the sag of Revit's triangles
there are printed for the first toilet (in id order) only, as Measure runs the
scorer with no options; `--list` prints them for every toilet.

Needs IfcOpenShell and numpy.

Usage:

    python3 tools/re/toilet_faces_vs_ifc.py <probe output> <revit.ifc> [--list]
"""

import collections
import os
import re
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from mesh_bodies_vs_ifc import distances, meshes, segment_distances, triangle_terms  # noqa: E402

FACE = re.compile(r"^face (\d+) owner (\d+) geometry (\d+) tag (-?\d+) surface (\S+) vertices (\d+) triangles (\d+) area (\S+)")
EDGES = (1e-4, 1e-3, 2e-3, 5e-3, 1e-2, 2e-2)


def parse(path):
    """Toilet id -> (face k -> (surface, area), triangles (T, 3, 3), face index per triangle)."""
    toilets = {}
    current = None
    for line in open(path, errors="replace"):
        parts = line.split()
        if not parts:
            continue
        if parts[0] == "toilet":
            current = toilets.setdefault(parts[1], ({}, [], []))
        elif parts[0] == "face" and current is not None:
            m = FACE.match(line)
            if m:
                current[0][int(m.group(1))] = (m.group(5), float(m.group(8)))
        elif parts[0] == "tri" and current is not None:
            current[1].append([float(x) for x in parts[2:11]])
            current[2].append(int(parts[1]))
    return {
        tag: (faces, np.array(tris).reshape(-1, 3, 3), np.array(owner))
        for tag, (faces, tris, owner) in toilets.items()
        if tris
    }


def nearest(points, terms, chunk=400):
    """Per point: the distance to, and the index of, the nearest triangle."""
    best = np.empty(len(points))
    arg = np.empty(len(points), dtype=np.int64)
    for start in range(0, len(points), chunk):
        d = distances(points[start : start + chunk][:, None, :], terms)
        arg[start : start + chunk] = d.argmin(axis=1)
        best[start : start + chunk] = d.min(axis=1)
    return best, arg


def boundary_edges(triangles, owner):
    """Face -> the edges (pairs of points) used by one triangle of the face only."""
    count = collections.Counter()
    edge = {}
    for t, face in zip(triangles, owner):
        keys = [tuple(np.round(v * 1e6).astype(np.int64)) for v in t]
        for i in range(3):
            a, b = keys[i], keys[(i + 1) % 3]
            if a == b:
                continue
            key = (int(face), min(a, b), max(a, b))
            count[key] += 1
            edge[key] = (t[i], t[(i + 1) % 3])
    out = collections.defaultdict(list)
    for key, n in count.items():
        if n == 1:
            out[key[0]].append(edge[key])
    return {k: np.array(v) for k, v in out.items()}


def explain(triangles, owner, faces, ref_triangles, ref_points, ref_terms, ours_terms,
            centres, centre_distance, vertex_distance, vertex_face):
    """Where Revit's vertices more than 0.001 ft from our surface lie, and what Revit's
    triangles are where our triangles lie more than 0.01 ft from them."""
    edges = boundary_edges(triangles, owner)
    off = np.flatnonzero(vertex_distance > 1e-3)
    print(f"  Revit's vertices off our surface by more than 0.001 ft: {len(off)} of {len(ref_points)}")
    print(
        f"  {'vertex':>6} {'dist':>7} {'face':>4} {'surface':<9} {'to edge':>8} {'next face':>9}"
        f" {'dist':>7} {'side':>5}  position"
    )
    a, unit = ours_terms[0], ours_terms[3]
    for i in off[np.argsort(-vertex_distance[off])]:
        p = ref_points[i]
        k = int(vertex_face[i])
        others = np.flatnonzero(owner != k)
        d_other, arg = nearest(p[None, :], tuple(x[others] for x in ours_terms))
        other = int(owner[others[arg[0]]])
        to_edge = float("inf")
        if k in edges:
            to_edge = float(segment_distances(p, edges[k][:, 0], edges[k][:, 1]).min())
        mine = np.flatnonzero(owner == k)
        _, arg_mine = nearest(p[None, :], tuple(x[mine] for x in ours_terms))
        t = mine[arg_mine[0]]
        side = float(np.dot(p - a[t], unit[t]))
        print(
            f"  {int(i):>6} {vertex_distance[i]:>7.4f} {k:>4} {faces[k][0]:<9} {to_edge:>8.4f}"
            f" {other:>9} {float(d_other[0]):>7.4f} {'+' if side > 0 else '-':>5}  "
            + " ".join(f"{x:.3f}" for x in p)
        )
    lengths = np.maximum.reduce(
        [np.linalg.norm(ref_triangles[:, i] - ref_triangles[:, (i + 1) % 3], axis=1) for i in range(3)]
    )
    far = np.flatnonzero(centre_distance > 1e-2)
    print(f"  our triangles with a centre more than 0.01 ft from Revit's surface: {len(far)} of {len(triangles)}")
    if not len(far):
        return
    _, arg = nearest(centres[far], ref_terms)
    order = np.argsort(-centre_distance[far])
    print(f"  {'face':>4} {'surface':<9} {'dist':>7} {'Revit edge':>10} {'radius':>8}  (radius = edge^2 / 8 dist)")
    for j in order[:24]:
        t, r = far[j], arg[j]
        d = centre_distance[t]
        print(
            f"  {int(owner[t]):>4} {faces[int(owner[t])][0]:<9} {d:>7.4f} {lengths[r]:>10.4f}"
            f" {lengths[r] ** 2 / (8 * d):>8.3f}"
        )
    sag = collections.defaultdict(list)
    for t, r in zip(far, arg):
        sag[int(owner[t])].append(lengths[r] ** 2 / (8 * centre_distance[t]))
    for k, v in sorted(sag.items()):
        print(
            f"  face {k} {faces[k][0]:<9}: {len(v)} triangles beyond 0.01 ft,"
            f" median radius edge^2 / 8 dist {np.median(v):.3f} ft"
        )


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    every = "--list" in sys.argv[1:]
    if len(args) != 2:
        print(__doc__)
        return 2
    ours = parse(args[0])
    print(f"toilets in the probe output: {len(ours)}")
    if not ours:
        return 0
    ref = meshes(args[1], tags=set(ours))
    listed = False
    for tag in sorted(ours):
        if tag not in ref:
            print(f"toilet {tag}: not in Revit's export")
            continue
        faces, triangles, owner = ours[tag]
        _, object_type, ref_triangles = ref[tag]
        ref_points = np.unique(ref_triangles.reshape(-1, 3), axis=0)
        ours_terms = triangle_terms(triangles)
        ref_terms = triangle_terms(ref_triangles)
        centres = triangles.mean(axis=1)
        areas = 0.5 * ours_terms[4]
        centre_distance, _ = nearest(centres, ref_terms)
        vertex_distance, vertex_face = nearest(ref_points, ours_terms)
        vertex_face = owner[vertex_face]
        print(
            f"toilet {tag} '{object_type}': ours {len(triangles)} triangles in {len(faces)} faces,"
            f" area {areas.sum():.4f}; Revit {len(ref_triangles)} triangles, {len(ref_points)} vertices,"
            f" area {0.5 * ref_terms[4].sum():.4f} sq ft"
        )
        print(
            "  centre of ours " + " ".join(f"{x:.3f}" for x in triangles.reshape(-1, 3).mean(axis=0))
            + ", of Revit's " + " ".join(f"{x:.3f}" for x in ref_triangles.reshape(-1, 3).mean(axis=0))
            + "; extent of ours " + " ".join(f"{x:.3f}" for x in np.ptp(triangles.reshape(-1, 3), axis=0))
            + ", of Revit's " + " ".join(f"{x:.3f}" for x in np.ptp(ref_triangles.reshape(-1, 3), axis=0))
        )
        histogram = np.searchsorted(EDGES, vertex_distance)
        labels = [f"<={e:g}" for e in EDGES] + [f">{EDGES[-1]:g}"]
        print(
            "  Revit's vertices to our surface: "
            + " ".join(f"{l}:{int((histogram == i).sum())}" for i, l in enumerate(labels))
        )
        by_class = collections.defaultdict(lambda: np.zeros(5))
        print(
            f"  {'face':>4} {'surface':<10} {'tris':>5} {'area':>8} {'>.001':>8} {'>.005':>8} {'>.01':>8}"
            f" {'max':>8} {'ref vtx':>7} {'>.001':>6} {'max':>8}"
        )
        for k in sorted(faces):
            surface, _ = faces[k]
            mask = owner == k
            if not mask.any():
                continue
            a, d = areas[mask], centre_distance[mask]
            near = vertex_face == k
            far = near & (vertex_distance > 1e-3)
            row = (
                a.sum(),
                a[d > 1e-3].sum(),
                a[d > 5e-3].sum(),
                a[d > 1e-2].sum(),
                float(d.max()),
            )
            by_class[surface] += np.array([*row[:4], int(far.sum())])
            print(
                f"  {k:>4} {surface:<10} {int(mask.sum()):>5} {row[0]:>8.4f} {row[1]:>8.4f} {row[2]:>8.4f}"
                f" {row[3]:>8.4f} {row[4]:>8.4f} {int(near.sum()):>7} {int(far.sum()):>6}"
                f" {(vertex_distance[far].max() if far.any() else 0.0):>8.4f}"
            )
        for surface, row in sorted(by_class.items()):
            print(
                f"  class {surface:<10} area {row[0]:.4f}, beyond .001 {row[1]:.4f},"
                f" beyond .005 {row[2]:.4f}, beyond .01 {row[3]:.4f}, Revit vertices off {int(row[4])}"
            )
        if listed and not every:
            continue
        listed = True
        explain(
            triangles, owner, faces, ref_triangles, ref_points, ref_terms, ours_terms,
            centres, centre_distance, vertex_distance, vertex_face,
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())

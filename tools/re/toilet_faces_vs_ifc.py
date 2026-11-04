#!/usr/bin/env python3
"""Score each decoded face of a toilet against Revit's mesh of the same toilet
(RE-302, #1247).

`mesh_bodies_vs_ifc.py` judges a whole body: the BIM4LCA office's 43
`WC-istuin:WC-1` toilets, drawn from their saved faces, are 1.027 to 1.047 of
Revit's volume, with our vertices up to 0.0218 ft from Revit's surface and
Revit's up to 0.0050 ft from ours. This reads `probe_re302_toilet_faces`'s
output (every decoded face of a few toilets, in document feet) and Revit's
export, and says, per face of ours:

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
from mesh_bodies_vs_ifc import distances, meshes, triangle_terms  # noqa: E402

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


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if len(args) != 2:
        print(__doc__)
        return 2
    ours = parse(args[0])
    print(f"toilets in the probe output: {len(ours)}")
    if not ours:
        return 0
    ref = meshes(args[1], tags=set(ours))
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
    return 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Set each face of a saved fitting mesh against Revit's own body (RE-311).

`probe_re311_fitting_faces` prints, for the elements it names (and every
element with a face that did not decode), the triangles of each face it
meshed in document feet as `tri <id> L<n> <oi>.<face>.<surface> x y z ...`.
This reads those at detail level 3 and the body Revit's export writes for the
same Tag, both in Revit's internal feet, and prints per element the volumes
(about each body's own centre, because a mesh with cracks between faces has
a volume that depends on where its origin is), then per face how far its
vertices lie from Revit's surface and how many of its triangles are wound the
way the nearest triangle of Revit's body is, and Revit's vertices that lie
more than 0.002 ft from the mesh with the face nearest each.

Needs IfcOpenShell and numpy.

Usage:

    python3 tools/re/fitting_faces_vs_ifc.py <probe output> <revit.ifc> [--list]
"""

import collections
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from mesh_bodies_vs_ifc import meshes  # noqa: E402

PAIR_BUDGET = 2_000_000


def read_probe(path):
    """id -> label -> (T, 3, 3) triangles at detail level 3."""
    out = collections.defaultdict(lambda: collections.defaultdict(list))
    with open(path, errors="replace") as handle:
        for line in handle:
            if not line.startswith("tri "):
                continue
            fields = line.split()
            if fields[2] != "L3" or len(fields) != 13:
                continue
            out[fields[1]][fields[3]].append([float(x) for x in fields[4:13]])
    return {
        tag: {label: np.array(rows, dtype=float).reshape(-1, 3, 3) for label, rows in faces.items()}
        for tag, faces in out.items()
    }


def signed_volume(triangles):
    centre = triangles.reshape(-1, 3).mean(axis=0)
    a, b, c = (triangles[:, k] - centre for k in range(3))
    return float(np.einsum("ij,ij->i", a, np.cross(b, c)).sum() / 6.0)


def segment_distances(p, a, b):
    ab = b - a
    t = np.einsum("ntk,tk->nt", p - a, ab) / np.maximum(np.einsum("tk,tk->t", ab, ab), 1e-24)
    q = a + np.clip(t, 0.0, 1.0)[..., None] * ab
    return np.linalg.norm(p - q, axis=-1)


def nearest(points, triangles):
    """Per point: the distance to the nearest triangle and that triangle's index."""
    a, b, c = triangles[:, 0], triangles[:, 1], triangles[:, 2]
    ab, ac = b - a, c - a
    normal = np.cross(ab, ac)
    length = np.linalg.norm(normal, axis=1)
    unit = normal / np.maximum(length, 1e-24)[:, None]
    d00 = np.einsum("tk,tk->t", ab, ab)
    d01 = np.einsum("tk,tk->t", ab, ac)
    d11 = np.einsum("tk,tk->t", ac, ac)
    denom = np.where(length > 1e-12, d00 * d11 - d01 * d01, 1.0)
    distances = np.empty(len(points))
    indices = np.empty(len(points), dtype=int)
    step = max(1, PAIR_BUDGET // len(triangles))
    for start in range(0, len(points), step):
        p = points[start : start + step][:, None, :]
        ap = p - a
        height = np.einsum("ntk,tk->nt", ap, unit)
        q = ap - height[..., None] * unit
        d20 = np.einsum("ntk,tk->nt", q, ab)
        d21 = np.einsum("ntk,tk->nt", q, ac)
        v = (d11 * d20 - d01 * d21) / denom
        w = (d00 * d21 - d01 * d20) / denom
        inside = (v >= -1e-9) & (w >= -1e-9) & (v + w <= 1 + 1e-9) & (length > 1e-12)
        plane = np.where(inside, np.abs(height), np.inf)
        best = np.minimum.reduce(
            [plane, segment_distances(p, a, b), segment_distances(p, b, c), segment_distances(p, c, a)]
        )
        distances[start : start + step] = best.min(axis=1)
        indices[start : start + step] = best.argmin(axis=1)
    return distances, indices


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        return 2
    probe = read_probe(sys.argv[1])
    ref = meshes(sys.argv[2], tags=set(probe))
    print(f"elements dumped {len(probe)}; in Revit's export {len(ref)}")
    for tag in sorted(set(probe) & set(ref), key=lambda t: (len(t), t)):
        faces = probe[tag]
        ours = np.concatenate(list(faces.values()))
        cls, object_type, theirs = ref[tag]
        v_ours, v_ref = signed_volume(ours), signed_volume(theirs)
        print(
            f"{tag} {cls} '{object_type}': rvt-rs {len(ours)} triangles, signed volume {v_ours:.6f};"
            f" Revit {len(theirs)} triangles, {v_ref:.6f}; ratio {abs(v_ours) / max(abs(v_ref), 1e-12):.4f}"
        )
        ref_points = np.unique(theirs.reshape(-1, 3), axis=0)
        ref_normals = np.cross(theirs[:, 1] - theirs[:, 0], theirs[:, 2] - theirs[:, 0])
        ref_normals /= np.maximum(np.linalg.norm(ref_normals, axis=1), 1e-24)[:, None]
        for label, tris in sorted(faces.items(), key=lambda kv: kv[0]):
            centroids = tris.mean(axis=1)
            vertices = np.unique(tris.reshape(-1, 3), axis=0)
            vertex_step = max(1, -(-len(vertices) // 600))
            d_vertices, _ = nearest(vertices[::vertex_step], theirs)
            d_centroids, which = nearest(centroids[:: max(1, -(-len(centroids) // 600))], theirs)
            sample = tris[:: max(1, -(-len(tris) // 600))]
            normals = np.cross(sample[:, 1] - sample[:, 0], sample[:, 2] - sample[:, 0])
            lengths = np.linalg.norm(normals, axis=1)
            keep = (lengths > 1e-12) & (d_centroids < 0.03)
            agree = (np.einsum("ij,ij->i", normals[keep] / lengths[keep][:, None], ref_normals[which[keep]]) > 0).sum()
            print(
                f"  face {label:<22} {len(tris):>5} tris, area {np.linalg.norm(np.cross(tris[:, 1] - tris[:, 0], tris[:, 2] - tris[:, 0]), axis=1).sum() / 2:.5f},"
                f" vertices to Revit's surface max {d_vertices.max():.5f} ft,"
                f" winding agrees with Revit's {agree} of {int(keep.sum())}"
            )
        d_ref, idx = nearest(ref_points, ours)
        far = np.argsort(-d_ref)[:8]
        for i in far:
            if d_ref[i] <= 0.002:
                break
            x, y, z = ref_points[i]
            print(f"  Revit vertex ({x:.4f}, {y:.4f}, {z:.4f}) is {d_ref[i]:.5f} ft from the mesh")
        within = int((d_ref <= 0.001).sum())
        print(f"  Revit vertices within 0.001 ft of the mesh: {within} of {len(ref_points)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

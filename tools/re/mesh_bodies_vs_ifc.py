#!/usr/bin/env python3
"""Score rvt-rs bodies drawn from Revit's saved display mesh against the
body Revit's own IFC export writes for the same element (W6, RE-78).

`saved_meshes_vs_ifc.py` compares only world boxes. This compares shapes:
for every element whose `BodySource` in the rvt-rs export is
`saved_graphics_mesh` and that Revit's export also holds (same `Tag`), it
meshes both in each file's own site frame, in Revit's internal feet, and
measures the volume and how far each body's vertices lie from the other's
surface. Per class, and under it per Revit type: how many have Revit's
volume within 1% and 0.01%, and every vertex of each body within 0.01 ft
and 0.001 ft of the other's surface. Then the elements furthest off.

Needs IfcOpenShell and numpy.

Usage:

    python3 tools/re/mesh_bodies_vs_ifc.py <rvt-rs.ifc> <revit.ifc>
"""

import collections
import os
import statistics
import sys

import ifcopenshell
import ifcopenshell.geom
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from opening_boxes_vs_ifc import site_frame  # noqa: E402
from roof_bodies_vs_ifc import shapes_of  # noqa: E402
from saved_meshes_vs_ifc import body_sources  # noqa: E402

SOURCE = "saved_graphics_mesh"
PAIR_BUDGET = 1_000_000


def meshes(path, tags=None, source=None):
    """Tag -> (class, ObjectType, triangles (T, 3, 3) in internal feet)."""
    model = ifcopenshell.open(path)
    frame = site_frame(model)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    out = {}
    for element in model.by_type("IfcElement"):
        if element.is_a("IfcOpeningElement") or not element.Tag:
            continue
        if tags is not None and element.Tag not in tags:
            continue
        if source is not None and body_sources(element) != source:
            continue
        parts = []
        for part in shapes_of(element):
            try:
                shape = ifcopenshell.geom.create_shape(settings, part)
            except RuntimeError:
                continue
            verts = np.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
            faces = np.array(shape.geometry.faces, dtype=int).reshape(-1, 3)
            if len(verts) and len(faces):
                parts.append(frame(verts)[faces])
        if not parts:
            continue
        triangles = np.concatenate(parts)
        if element.Tag in out:
            triangles = np.concatenate([out[element.Tag][2], triangles])
        out[element.Tag] = (element.is_a(), element.ObjectType or "", triangles)
    return out


def volume(triangles):
    a, b, c = triangles[:, 0], triangles[:, 1], triangles[:, 2]
    return abs(np.einsum("ij,ij->i", a, np.cross(b, c)).sum() / 6.0)


def segment_distances(p, a, b):
    ab = b - a
    t = np.einsum("ntk,tk->nt", p - a, ab) / np.maximum(np.einsum("tk,tk->t", ab, ab), 1e-24)
    q = a + np.clip(t, 0.0, 1.0)[..., None] * ab
    return np.linalg.norm(p - q, axis=-1)


def surface_distance(points, triangles):
    """The largest distance from any of `points` to the nearest triangle."""
    a, b, c = triangles[:, 0], triangles[:, 1], triangles[:, 2]
    ab, ac = b - a, c - a
    normal = np.cross(ab, ac)
    length = np.linalg.norm(normal, axis=1)
    unit = normal / np.maximum(length, 1e-24)[:, None]
    d00 = np.einsum("tk,tk->t", ab, ab)
    d01 = np.einsum("tk,tk->t", ab, ac)
    d11 = np.einsum("tk,tk->t", ac, ac)
    denom = np.where(length > 1e-12, d00 * d11 - d01 * d01, 1.0)
    worst = 0.0
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
        nearest = np.minimum.reduce(
            [plane, segment_distances(p, a, b), segment_distances(p, b, c), segment_distances(p, c, a)]
        ).min(axis=1)
        worst = max(worst, float(nearest.max()))
    return worst


def main():
    if len(sys.argv) != 3:
        print(__doc__)
        return 2
    ours = meshes(sys.argv[1], source=SOURCE)
    ref = meshes(sys.argv[2], tags=set(ours))
    print(f"{SOURCE} bodies: {len(ours)}; in Revit's export {len(ref)}")
    rows = collections.defaultdict(lambda: collections.defaultdict(collections.Counter))
    ratios = collections.defaultdict(list)
    scored = []
    for tag in sorted(set(ours) & set(ref), key=lambda t: (len(t), t)):
        _, _, our_triangles = ours[tag]
        cls, object_type, ref_triangles = ref[tag]
        ref_volume, our_volume = volume(ref_triangles), volume(our_triangles)
        ratio = our_volume / ref_volume if ref_volume > 0 else float("inf")
        to_ours = surface_distance(np.unique(ref_triangles.reshape(-1, 3), axis=0), our_triangles)
        to_ref = surface_distance(np.unique(our_triangles.reshape(-1, 3), axis=0), ref_triangles)
        far = max(to_ours, to_ref)
        for key in ("", object_type):
            row = rows[cls][key]
            row["n"] += 1
            row["vol 1%"] += abs(ratio - 1) <= 0.01
            row["vol 0.01%"] += abs(ratio - 1) <= 1e-4
            row["0.01 ft"] += far <= 0.01
            row["0.001 ft"] += far <= 0.001
        ratios[cls].append(ratio)
        scored.append((far, tag, cls, object_type, ratio, to_ours, to_ref))
    print(f"{'class':<28} {'n':>6} {'vol 1%':>7} {'vol .01%':>8} {'0.01 ft':>8} {'0.001 ft':>8}  median volume ratio")
    for cls in sorted(rows, key=lambda c: -rows[c][""]["n"]):
        row = rows[cls][""]
        print(
            f"{cls:<28} {row['n']:>6} {row['vol 1%']:>7} {row['vol 0.01%']:>8} {row['0.01 ft']:>8} {row['0.001 ft']:>8}"
            f"  {statistics.median(ratios[cls]):.4f}"
        )
        types = sorted((k for k in rows[cls] if k), key=lambda k: -rows[cls][k]["n"])
        for key in types[:6]:
            t = rows[cls][key]
            print(f"    {t['n']:>5} {t['vol 1%']:>7} {t['vol 0.01%']:>8} {t['0.01 ft']:>8} {t['0.001 ft']:>8}  {key}")
    scored.sort(reverse=True)
    for far, tag, cls, object_type, ratio, to_ours, to_ref in scored[:15]:
        print(
            f"  {tag} {cls} '{object_type}': volume ratio {ratio:.4f}, Revit's vertices to rvt-rs's surface"
            f" {to_ours:.4g} ft, rvt-rs's to Revit's {to_ref:.4g} ft"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Which simple measures of a body rvt-rs draws from a saved mesh are the
quantities Revit's export writes for it (#1046, W7).

For each object whose Body in the rvt-rs export is an `IfcTriangulatedFaceSet`,
this works out measures of its mesh in the element's own frame, in metres:
its volume and surface area, the extents of its box along the element's axes
and sorted, the sides of the smallest rectangle around its plan, the areas of
its shadow on each axis plane, the area of its largest planar face (and that
face's outer loop) and their perimeters, and the products of two extents. For
each quantity of the object's `Qto_` sets Revit's export writes, it counts how
many of the measures of the quantity's kind (length, area, volume) are equal to
it (the scorers' tolerance: 1e-6 SI or a relative 1e-5), by entity and
quantity. It scores nothing and changes nothing: it says which measure to read
a quantity from before one is written.

It prints, per entity and quantity:

  <entity> <set>.<name>: Revit writes N on an object drawn from a mesh; rvt-rs
      writes it on W (equal to Revit's on V); equal to a measure on A of N:
      <measure> E, <measure> E, ...; none written, some measure equal on K of M

then up to 3 objects where no measure is equal, with Revit's value and the
measure nearest it. Usage:

    python3 tools/re/mesh_features_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>
    python3 tools/re/mesh_features_vs_ifc.py <rvt-rs.ifc> <rvt-rs.ifc>   # a self-check

Needs IfcOpenShell and numpy (the versions tools/ci/requirements-ifc.txt pins).
"""

import collections
import multiprocessing
import os
import sys

import ifcopenshell
import ifcopenshell.geom
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import qtos_vs_ifc  # noqa: E402
from mesh_quantities_vs_ifc import mesh_bodies  # noqa: E402

# Above this many triangles a body's faces are not grouped: its other measures are still scored.
MAX_FACE_TRIANGLES = 20000
LENGTH = (
    "dx", "dy", "dz", "e_min", "e_mid", "e_max", "obb_long", "obb_short",
    "lf_perimeter", "lf_perimeter_outer",
)
AREA = (
    "area", "half_area", "proj_xy", "proj_xz", "proj_yz", "lf_area", "lf_area_outer",
    "dx_dy", "dx_dz", "dy_dz", "emin_emid", "emin_emax", "emid_emax",
)
VOLUME = ("volume", "box_volume")
CANDIDATES = {"IfcQuantityLength": LENGTH, "IfcQuantityArea": AREA, "IfcQuantityVolume": VOLUME}


def convex_hull(points):
    """Andrew's monotone chain over unique 2D points, counter-clockwise."""
    pts = sorted(map(tuple, points))
    if len(pts) < 3:
        return np.array(pts)

    def cross(o, a, b):
        return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])

    lower, upper = [], []
    for p in pts:
        while len(lower) >= 2 and cross(lower[-2], lower[-1], p) <= 0:
            lower.pop()
        lower.append(p)
    for p in reversed(pts):
        while len(upper) >= 2 and cross(upper[-2], upper[-1], p) <= 0:
            upper.pop()
        upper.append(p)
    return np.array(lower[:-1] + upper[:-1])


def plan_rectangle(verts):
    """(long side, short side) of the smallest rectangle around the plan of `verts`."""
    flat = np.unique(np.round(verts[:, :2], 9), axis=0)
    if len(flat) < 3:
        return None
    hull = convex_hull(flat)
    if len(hull) < 3:
        return None
    best = None
    for i in range(len(hull)):
        edge = hull[(i + 1) % len(hull)] - hull[i]
        norm = float(np.hypot(edge[0], edge[1]))
        if norm == 0:
            continue
        u = edge / norm
        v = np.array([-u[1], u[0]])
        a, b = hull @ u, hull @ v
        w, h = float(a.max() - a.min()), float(b.max() - b.min())
        if best is None or w * h < best[0]:
            best = (w * h, w, h)
    return None if best is None else (max(best[1], best[2]), min(best[1], best[2]))


def largest_face(verts, tris):
    """(area, outer loop area, perimeter, outer loop perimeter) of the largest
    planar face of the mesh, or None where its loops do not chain."""
    corners = verts[tris]
    cross = np.cross(corners[:, 1] - corners[:, 0], corners[:, 2] - corners[:, 0])
    size = np.linalg.norm(cross, axis=1)
    live = np.flatnonzero(size > 1e-12)
    if not len(live):
        return None
    unit = cross[live] / size[live, None]
    offset = np.einsum("ij,ij->i", unit, corners[live, 0])
    key = np.round(np.hstack([unit, offset[:, None]]), 5)
    _, group = np.unique(key, axis=0, return_inverse=True)
    group = group.reshape(-1)
    welded = np.round(verts * 1e6).astype(np.int64)
    _, vid = np.unique(welded, axis=0, return_inverse=True)
    vid = vid.reshape(-1)
    parent = list(range(len(live)))

    def find(i):
        while parent[i] != i:
            parent[i] = parent[parent[i]]
            i = parent[i]
        return i

    edge_owner = {}
    for local, tri in enumerate(live):
        a, b, c = (int(vid[t]) for t in tris[tri])
        for e in ((a, b), (b, c), (c, a)):
            edge = (min(e), max(e))
            seen = edge_owner.get((int(group[local]), edge))
            if seen is None:
                edge_owner[(int(group[local]), edge)] = local
            else:
                ra, rb = find(seen), find(local)
                if ra != rb:
                    parent[ra] = rb
    area_of = collections.Counter()
    for local in range(len(live)):
        area_of[find(local)] += float(size[live[local]]) / 2
    root = max(area_of, key=area_of.get)
    members = [local for local in range(len(live)) if find(local) == root]
    edges = collections.Counter()
    for local in members:
        a, b, c = (int(vid[t]) for t in tris[live[local]])
        for e in ((a, b), (b, c), (c, a)):
            edges[(min(e), max(e))] += 1
    boundary = [e for e, n in edges.items() if n == 1]
    where = {}
    for tri in tris[live[members]].reshape(-1):
        where.setdefault(int(vid[tri]), verts[tri])
    adjacency = collections.defaultdict(list)
    for a, b in boundary:
        adjacency[a].append(b)
        adjacency[b].append(a)
    if any(len(n) != 2 for n in adjacency.values()):
        return None
    normal = unit[members[0]]
    loops = []
    visited = set()
    for start in adjacency:
        if start in visited:
            continue
        loop, here, before = [start], start, None
        while True:
            visited.add(here)
            nxt = [n for n in adjacency[here] if n != before]
            nxt = nxt[0] if nxt else adjacency[here][0]
            if nxt == start:
                break
            if nxt in visited:
                return None
            loop.append(nxt)
            before, here = here, nxt
        points = np.array([where[v] for v in loop])
        closed = np.vstack([points, points[:1]])
        length = float(np.linalg.norm(np.diff(closed, axis=0), axis=1).sum())
        twice = np.cross(points, np.roll(points, -1, axis=0)).sum(axis=0)
        loops.append((abs(float(twice @ normal)) / 2, length))
    outer = max(loops)
    return area_of[root], outer[0], sum(length for _, length in loops), outer[1]


def measures(verts, tris):
    """Name -> value (SI) of every candidate measure of a mesh in its own frame."""
    corners = verts[tris]
    cross = np.cross(corners[:, 1] - corners[:, 0], corners[:, 2] - corners[:, 0])
    size = np.linalg.norm(cross, axis=1)
    centre = corners.reshape(-1, 3).mean(axis=0)
    a, b, c = corners[:, 0] - centre, corners[:, 1] - centre, corners[:, 2] - centre
    out = {
        "volume": abs(float(np.einsum("ij,ij->i", a, np.cross(b, c)).sum())) / 6,
        "area": float(size.sum() / 2),
    }
    out["half_area"] = out["area"] / 2
    with np.errstate(invalid="ignore", divide="ignore"):
        unit = np.where(size[:, None] > 0, cross / size[:, None], 0.0)
    for axis, name in ((2, "proj_xy"), (1, "proj_xz"), (0, "proj_yz")):
        out[name] = float((size * np.abs(unit[:, axis])).sum() / 4)
    extent = verts.max(axis=0) - verts.min(axis=0)
    out.update(dx=float(extent[0]), dy=float(extent[1]), dz=float(extent[2]))
    low, mid, high = sorted(float(e) for e in extent)
    out.update(e_min=low, e_mid=mid, e_max=high)
    out.update(
        dx_dy=float(extent[0] * extent[1]),
        dx_dz=float(extent[0] * extent[2]),
        dy_dz=float(extent[1] * extent[2]),
        emin_emid=low * mid,
        emin_emax=low * high,
        emid_emax=mid * high,
        box_volume=low * mid * high,
    )
    if len(tris) <= MAX_FACE_TRIANGLES:
        rectangle = plan_rectangle(verts)
        if rectangle is not None:
            out["obb_long"], out["obb_short"] = rectangle
        face = largest_face(verts, tris)
        if face is not None:
            out["lf_area"], out["lf_area_outer"], out["lf_perimeter"], out["lf_perimeter_outer"] = face
    return out


def mesh_measures(f, products):
    """key -> measures, each body in its own frame (IfcOpenShell's default settings)."""
    settings = ifcopenshell.geom.settings()
    by_id = {product.id(): key for key, product in products.items()}
    out = {}
    iterator = ifcopenshell.geom.iterator(settings, f, multiprocessing.cpu_count(), include=list(products.values()))
    if not iterator.initialize():
        return out
    while True:
        shape = iterator.get()
        key = by_id.get(shape.id)
        verts = np.asarray(shape.geometry.verts, dtype=float).reshape(-1, 3)
        tris = np.asarray(shape.geometry.faces, dtype=int).reshape(-1, 3)
        if key is not None and len(tris):
            out[key] = measures(verts, tris)
        if not iterator.next():
            break
    return out


def main(argv):
    args = [a for a in argv[1:] if not a.startswith("--")]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    ours_path, theirs_path = args
    ours_file = ifcopenshell.open(ours_path)
    ours, _, our_entities, _, _, _ = qtos_vs_ifc.quantities(ours_path)
    theirs, _, their_entities, _, _, _ = qtos_vs_ifc.quantities(theirs_path, our_entities)
    bodies = mesh_bodies(ours_file)
    print(f"objects rvt-rs draws from a mesh: {len(bodies)}")
    values = mesh_measures(ours_file, bodies)
    rows = collections.defaultdict(collections.Counter)
    hits = collections.defaultdict(collections.Counter)
    examples = collections.defaultdict(list)
    for (key, set_name, name), (kind, revit_value) in sorted(theirs.items()):
        names = CANDIDATES.get(kind)
        if names is None or key not in values or revit_value is None:
            continue
        entity = our_entities.get(key, "?")
        label = (entity, set_name, name)
        row = rows[label]
        row["revit"] += 1
        mine = ours.get((key, set_name, name))
        if mine is None:
            row["none written"] += 1
        else:
            row["written"] += 1
            if qtos_vs_ifc.same((kind, revit_value), mine):
                row["written equal"] += 1
        equal = [n for n in names if n in values[key] and qtos_vs_ifc.same((kind, revit_value), (kind, values[key][n]))]
        for n in equal:
            hits[label][n] += 1
        if equal:
            row["some measure equal"] += 1
            if mine is None:
                row["none written, some measure equal"] += 1
        elif len(examples[label]) < 3:
            near = min(
                (n for n in names if n in values[key]),
                key=lambda n: abs(values[key][n] - revit_value),
                default=None,
            )
            examples[label].append((key, revit_value, near, None if near is None else values[key][near]))
    for label, row in sorted(rows.items()):
        entity, set_name, name = label
        best = ", ".join(f"{n} {c}" for n, c in hits[label].most_common(5)) or "none"
        print(
            f"{entity} {set_name}.{name}: Revit writes {row['revit']} on an object drawn from a mesh; "
            f"rvt-rs writes it on {row['written']} (equal to Revit's on {row['written equal']}); "
            f"equal to a measure on {row['some measure equal']} of {row['revit']}: {best}; "
            f"none written, some measure equal on {row['none written, some measure equal']} of {row['none written']}"
        )
    for label, items in sorted(examples.items()):
        for key, revit_value, near, value in items:
            shown = "no measure" if near is None else f"nearest {near} {value:.6g}"
            print(f"  no measure equal {label[0]} {label[1]}.{label[2]} {key}: Revit {revit_value:.6g}, {shown}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

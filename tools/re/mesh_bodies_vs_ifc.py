#!/usr/bin/env python3
"""Score rvt-rs bodies drawn from Revit's saved display mesh against the
body Revit's own IFC export writes for the same element (W6, RE-78).

`saved_meshes_vs_ifc.py` compares only world boxes. This compares shapes:
for every element whose `BodySource` in the rvt-rs export is
`saved_graphics_mesh` and that Revit's export also holds (same `Tag`), it
meshes both in each file's own site frame, in Revit's internal feet, and
measures the volume and how far each body's vertices lie from the other's
surface. Per class, and under it per Revit type: how many have Revit's
volume within 1% and 0.01%, every vertex of each body within 0.01 ft
and 0.001 ft of the other's surface, and "ref on ours": every tested vertex
of Revit's body within 0.001 ft of rvt-rs's surface, which tells a mesh of
the same surface from a mesh of another one where Revit's export is the
coarser (RE-311). Then the elements furthest off.

A class with more than 300 such bodies is scored on an even stride of 300 of
them by Tag, and at most 400 vertices of each body are tested, so a file
with thousands of mullions fits the time Measure allows. The bodies are
scored one process each over the CPUs (RVT_MESH_WORKERS sets the count), by
the same arithmetic wherever a body runs, so the output does not depend on
it. Bodies only exist in an export made with `--saved-meshes`; without it
this prints none.

Needs IfcOpenShell and numpy.

Usage:

    python3 tools/re/mesh_bodies_vs_ifc.py <rvt-rs.ifc> <revit.ifc>
"""

import collections
import concurrent.futures
import concurrent.futures.process
import multiprocessing
import os
import statistics
import sys

import ifcopenshell
import ifcopenshell.geom
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ifc_common import mesh_parts, site_frame  # noqa: E402
from roof_bodies_vs_ifc import shapes_of  # noqa: E402
from saved_meshes_vs_ifc import body_sources  # noqa: E402

SOURCE = "saved_graphics_mesh"
PAIR_BUDGET = 1_000_000
# surface_distance measures every pair up to this many, and beyond it prunes.
EXHAUSTIVE = 200_000
BLOCK = 8
SEEDS = 4
SLIVER = 1e-2
# Most elements scored per class (an even stride through them by Tag), and
# most vertices of a body tested against the other's surface, so a file
# with thousands of mullions is scored in the time Measure allows.
CLASS_CAP = 300
VERTEX_CAP = 400
# Fewer bodies than this are scored in this process: a forked worker has a
# cost of its own that a handful do not repay.
POOL_FROM = 16


def meshes(path, tags=None, source=None, model=None):
    """Tag -> (class, ObjectType, triangles (T, 3, 3) in internal feet)."""
    if model is None:
        model = ifcopenshell.open(path)
    frame = site_frame(model)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    chosen = []
    for element in model.by_type("IfcElement"):
        if element.is_a("IfcOpeningElement") or not element.Tag:
            continue
        if tags is not None and element.Tag not in tags:
            continue
        if source is not None and body_sources(element) != source:
            continue
        chosen.append((element, shapes_of(element)))
    shapes = mesh_parts(model, settings, [part for _, parts in chosen for part in parts], path)
    out = {}
    for element, parts in chosen:
        triangles = []
        for part in parts:
            if part.id() not in shapes:
                continue
            verts, faces = shapes[part.id()]
            if len(verts) and len(faces):
                triangles.append(frame(verts)[faces])
        if not triangles:
            continue
        triangles = np.concatenate(triangles)
        if element.Tag in out:
            triangles = np.concatenate([out[element.Tag][2], triangles])
        out[element.Tag] = (element.is_a(), element.ObjectType or "", triangles)
    return out


def volume(triangles):
    """The enclosed volume, taken about the body's own centre: a mesh with
    cracks between its faces (each curved face is meshed on its own) has a
    signed volume that grows with its distance from the origin, as a closed
    mesh's does not (RE-311). The centre takes away only the origin's part:
    a body missing a face still differs by its open boundary's area vector
    times the centre's distance from it."""
    centre = triangles.reshape(-1, 3).mean(axis=0)
    a, b, c = triangles[:, 0] - centre, triangles[:, 1] - centre, triangles[:, 2] - centre
    return abs(np.einsum("ij,ij->i", a, np.cross(b, c)).sum() / 6.0)


def shell_volumes(triangles):
    """The signed volume of each set of triangles joined through shared
    vertices (to 1e-6 ft), largest first."""
    keys = [tuple(np.round(v * 1e6).astype(np.int64)) for v in triangles.reshape(-1, 3)]
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
    centre = triangles.reshape(-1, 3).mean(axis=0)
    a, b, c = triangles[:, 0] - centre, triangles[:, 1] - centre, triangles[:, 2] - centre
    signed = np.einsum("ij,ij->i", a, np.cross(b, c)) / 6.0
    shells = collections.defaultdict(float)
    for i, v in enumerate(signed):
        shells[find(keys[3 * i])] += float(v)
    return sorted(shells.values(), key=lambda v: -abs(v))


def dot(x, y):
    return np.einsum("...k,...k->...", x, y)


def segment_distances(p, a, b):
    ab = b - a
    t = dot(p - a, ab) / np.maximum(dot(ab, ab), 1e-24)
    q = a + np.clip(t, 0.0, 1.0)[..., None] * ab
    return np.linalg.norm(p - q, axis=-1)


def triangle_terms(triangles):
    a, b, c = triangles[:, 0], triangles[:, 1], triangles[:, 2]
    ab, ac = b - a, c - a
    normal = np.cross(ab, ac)
    length = np.linalg.norm(normal, axis=1)
    unit = normal / np.maximum(length, 1e-24)[:, None]
    d00, d01, d11 = dot(ab, ab), dot(ab, ac), dot(ac, ac)
    denom = np.where(length > 1e-12, d00 * d11 - d01 * d01, 1.0)
    return a, b, c, unit, length, d00, d01, d11, denom


def distances(p, terms):
    """Distance from points `p` (..., 1, 3) to triangles whose terms broadcast with them."""
    a, b, c, unit, length, d00, d01, d11, denom = terms
    ab, ac = b - a, c - a
    ap = p - a
    height = dot(ap, unit)
    q = ap - height[..., None] * unit
    d20 = dot(q, ab)
    d21 = dot(q, ac)
    v = (d11 * d20 - d01 * d21) / denom
    w = (d00 * d21 - d01 * d20) / denom
    inside = (v >= -1e-9) & (w >= -1e-9) & (v + w <= 1 + 1e-9) & (length > 1e-12)
    plane = np.where(inside, np.abs(height), np.inf)
    return np.minimum.reduce([plane, segment_distances(p, a, b), segment_distances(p, b, c), segment_distances(p, c, a)])


def morton(centres):
    """The order of points along a Z-order curve, so triangles near each other are near in the order."""
    lo, hi = centres.min(axis=0), centres.max(axis=0)
    cells = ((centres - lo) / np.maximum(hi - lo, 1e-12) * 1023).astype(np.uint64)
    code = np.zeros(len(centres), dtype=np.uint64)
    for bit in range(10):
        for axis in range(3):
            code |= ((cells[:, axis] >> np.uint64(bit)) & np.uint64(1)) << np.uint64(3 * bit + axis)
    return np.argsort(code, kind="stable")


def surface_distance(points, triangles):
    """The largest distance from any of `points` to the nearest triangle.

    Exact, not approximate: for each point the nearest distance is the minimum of
    `distances` over a set of triangles that holds every one that could be nearest,
    and each of them is measured by the very arithmetic that measures all of them in
    a small job (`EXHAUSTIVE` pairs or fewer, and any input that is not finite), so
    the result is the same float.

    A triangle is left out for a point only when a lower bound on its distance
    exceeds the nearest distance already found for that point plus a slack. The
    slack is 1e-6 ft and 4e-9 of the extent, the largest of any coordinate and any
    triangle edge. It has to cover two things. `inside` takes a point up to 1e-9 of
    two edges outside the triangle in its plane (or 1e-9 past the third) as inside
    and measures it by its plane alone, so the distance computed for it falls short
    of the distance to the triangle, which the bounds are on, by up to 1e-9 times
    the sum of two edges: under 2e-9 of the extent. And the rounding of these
    distances, which is many orders of magnitude smaller.

    The bounds hold for every triangle but the slivers (the sine of the angle between
    two edges under SLIVER, whose computed plane is not reliable) and the zero-area
    ones (which have no plane, so only their box bounds them): the distance to a
    block of eight neighbouring triangles' box, then to a triangle's box, and for a
    triangle with a plane the distance to its plane combined with that to the side
    planes of its edges. A sliver's `inside` cannot be trusted, so a point is measured
    against it unless the point is further than the nearest distance found plus the
    slack from both its box (which bounds the distances to its edges) and its plane
    (`distances` gives the plane's distance only for a point it takes as `inside`,
    and then as exactly this height).
    """
    terms = triangle_terms(triangles)
    n = len(points)
    if n * len(triangles) <= EXHAUSTIVE or not (np.isfinite(points).all() and np.isfinite(triangles).all()):
        worst = 0.0
        step = max(1, PAIR_BUDGET // len(triangles))
        for start in range(0, n, step):
            nearest = distances(points[start : start + step][:, None, :], terms).min(axis=1)
            worst = max(worst, float(nearest.max()))
        return worst
    a, b, c, unit, length, d00, d01, d11, denom = terms
    sliver = (length > 1e-12) & (length < SLIVER * np.sqrt(d00 * d11))
    flat = (length > 1e-12) & ~sliver
    lo = np.minimum(np.minimum(a, b), c)
    hi = np.maximum(np.maximum(a, b), c)
    extent = max(float(np.abs(triangles).max()), float(np.abs(points).max()), float(np.sqrt(max(d00.max(), d11.max()))))
    slack = 1e-6 + 4e-9 * extent
    sides = []
    for start, end in ((a, b), (b, c), (c, a)):
        out = np.cross(end - start, unit)
        sides.append((start, out / np.maximum(np.linalg.norm(out, axis=1), 1e-24)[:, None]))
    order = morton((lo + hi) / 2)
    order = np.concatenate([order, np.full(-len(order) % BLOCK, order[-1])])
    blocks = order.reshape(-1, BLOCK)
    block_lo, block_hi = lo[blocks].min(axis=1), hi[blocks].max(axis=1)
    seeds = min(SEEDS, len(blocks))
    nearest = np.empty(n)
    step = max(1, PAIR_BUDGET // len(blocks))
    for start in range(0, n, step):
        p = points[start : start + step]
        gap = np.maximum(np.maximum(block_lo[None] - p[:, None], p[:, None] - block_hi[None]), 0.0)
        bound = np.sqrt(dot(gap, gap))
        rows = np.arange(len(p))
        seed = np.argpartition(bound, seeds - 1, axis=1)[:, :seeds] if seeds < len(blocks) else np.tile(np.arange(seeds), (len(p), 1))
        best = np.full(len(p), np.inf)
        for column in range(seeds):
            best = np.minimum(best, distances(p[:, None, :], tuple(x[blocks[seed[:, column]]] for x in terms)).min(axis=1))
            bound[rows, seed[:, column]] = np.inf
        pair_rows, pair_blocks = np.nonzero(bound <= (best + slack)[:, None])
        done = 0
        while done < len(pair_rows):
            stop = int(np.searchsorted(pair_rows, pair_rows[min(len(pair_rows) - 1, done + PAIR_BUDGET // BLOCK)], side="right"))
            row = np.repeat(pair_rows[done:stop], BLOCK)
            tri = blocks[pair_blocks[done:stop]].reshape(-1)
            done = stop
            q = p[row]
            gap = np.maximum(np.maximum(lo[tri] - q, q - hi[tri]), 0.0)
            floor = np.sqrt(dot(gap, gap))
            height = dot(q - a[tri], unit[tri])
            outside = np.zeros(len(tri))
            for corner, side in sides:
                outside = np.maximum(outside, dot(q - corner[tri], side[tri]))
            floor = np.where(flat[tri], np.maximum(floor, np.sqrt(height * height + outside * outside)), floor)
            first = np.flatnonzero(np.r_[True, row[1:] != row[:-1]])
            lowest = np.minimum.reduceat(floor, first)
            group = np.repeat(np.arange(len(first)), np.diff(np.r_[first, len(row)]))
            pick = np.flatnonzero(floor == lowest[group])
            pick = pick[np.unique(group[pick], return_index=True)[1]]
            best[row[pick]] = np.minimum(best[row[pick]], distances(q[pick][:, None, :], tuple(x[tri[pick]][:, None] for x in terms))[:, 0])
            keep = floor <= best[row] + slack
            row, tri = row[keep], tri[keep]
            np.minimum.at(best, row, distances(p[row][:, None, :], tuple(x[tri][:, None] for x in terms))[:, 0])
        nearest[start : start + step] = best
    slivers = np.flatnonzero(sliver)
    if len(slivers):
        near = tuple(x[slivers] for x in terms)
        near_lo, near_hi = lo[slivers], hi[slivers]
        step = max(1, (PAIR_BUDGET // 4) // len(slivers))
        for start in range(0, n, step):
            p = points[start : start + step]
            held = nearest[start : start + step]
            gap = np.maximum(np.maximum(near_lo[None] - p[:, None], p[:, None] - near_hi[None]), 0.0)
            floor = np.minimum(np.sqrt(dot(gap, gap)), np.abs(dot(p[:, None, :] - near[0][None], near[3][None])))
            row, col = np.nonzero(floor <= (held + slack)[:, None])
            for done in range(0, len(row), PAIR_BUDGET):
                r, c = row[done : done + PAIR_BUDGET], col[done : done + PAIR_BUDGET]
                np.minimum.at(held, r, distances(p[r][:, None, :], tuple(x[c][:, None] for x in near))[:, 0])
    return float(nearest.max())


def sample(triangles):
    """The body's distinct vertices in lexicographic order, every `step`th of them so that
    at most VERTEX_CAP are kept (what `np.unique(axis=0)` gives, sorted without its
    structured-array comparison)."""
    vertices = triangles.reshape(-1, 3)
    vertices = vertices[np.lexsort((vertices[:, 2], vertices[:, 1], vertices[:, 0]))]
    points = vertices[np.r_[True, (vertices[1:] != vertices[:-1]).any(axis=1)]]
    step = max(1, -(-len(points) // VERTEX_CAP))
    return points[::step]


SCORING = {}


def score_pair(tag):
    """(volume ratio, Revit's vertices to rvt-rs's surface, rvt-rs's to Revit's) for one body."""
    our_triangles, ref_triangles = SCORING["ours"][tag][2], SCORING["ref"][tag][2]
    ref_volume, our_volume = volume(ref_triangles), volume(our_triangles)
    ratio = our_volume / ref_volume if ref_volume > 0 else float("inf")
    to_ours = surface_distance(sample(ref_triangles), our_triangles)
    to_ref = surface_distance(sample(our_triangles), ref_triangles)
    return ratio, to_ours, to_ref


def score_all(ours, ref, tags):
    """`score_pair` of each of `tags`, in their order, spread over the CPUs:
    each body is scored alone, by the same arithmetic wherever it runs, so
    what is printed does not depend on the number of workers."""
    SCORING.update(ours=ours, ref=ref)
    cpus = len(os.sched_getaffinity(0)) if hasattr(os, "sched_getaffinity") else os.cpu_count() or 1
    workers = min(int(os.environ.get("RVT_MESH_WORKERS") or cpus), len(tags))
    results = None
    if workers > 1 and len(tags) >= POOL_FROM:
        # The heaviest first, so the last body does not run alone. A forked
        # worker flushes the buffered output it inherits when it ends.
        order = sorted(range(len(tags)), key=lambda i: -(len(ours[tags[i]][2]) + len(ref[tags[i]][2])))
        sys.stdout.flush()
        sys.stderr.flush()
        try:
            with concurrent.futures.ProcessPoolExecutor(workers, mp_context=multiprocessing.get_context("fork")) as pool:
                done = list(pool.map(score_pair, [tags[i] for i in order], chunksize=1))
            results = [None] * len(tags)
            for i, result in zip(order, done):
                results[i] = result
        except concurrent.futures.process.BrokenProcessPool:
            print("mesh_bodies_vs_ifc: a worker died, so the bodies are scored one after another", file=sys.stderr)
            results = None
    if results is None:
        results = [score_pair(tag) for tag in tags]
    SCORING.clear()
    return results


def main():
    if len(sys.argv) != 3:
        print(__doc__)
        return 2
    model = ifcopenshell.open(sys.argv[1])
    by_class = collections.defaultdict(list)
    for element in model.by_type("IfcElement"):
        if element.Tag and not element.is_a("IfcOpeningElement") and body_sources(element) == SOURCE:
            by_class[element.is_a()].append(element.Tag)
    chosen = set()
    for cls, tags in by_class.items():
        tags = sorted(set(tags), key=lambda t: (len(t), t))
        step = max(1, -(-len(tags) // CLASS_CAP))
        chosen.update(tags[::step])
        if step > 1:
            print(f"{cls}: {len(tags)} bodies, every {step}th scored")
    ours = meshes(sys.argv[1], tags=chosen, source=SOURCE, model=model)
    del model
    ref = meshes(sys.argv[2], tags=set(ours))
    print(f"{SOURCE} bodies scored: {len(ours)}; in Revit's export {len(ref)}")
    rows = collections.defaultdict(lambda: collections.defaultdict(collections.Counter))
    ratios = collections.defaultdict(list)
    scored = []
    tags = sorted(set(ours) & set(ref), key=lambda t: (len(t), t))
    for tag, (ratio, to_ours, to_ref) in zip(tags, score_all(ours, ref, tags)):
        cls, object_type, _ = ref[tag]
        far = max(to_ours, to_ref)
        for key in ("", object_type) if object_type else ("",):
            row = rows[cls][key]
            row["n"] += 1
            row["vol 1%"] += abs(ratio - 1) <= 0.01
            row["vol 0.01%"] += abs(ratio - 1) <= 1e-4
            row["0.01 ft"] += far <= 0.01
            row["0.001 ft"] += far <= 0.001
            row["ref on ours"] += to_ours <= 0.001
        ratios[cls].append(ratio)
        scored.append((far, tag, cls, object_type, ratio, to_ours, to_ref))
    print(
        f"{'class':<28} {'n':>6} {'vol 1%':>7} {'vol .01%':>8} {'0.01 ft':>8} {'0.001 ft':>8}"
        f" {'ref on ours':>11}  median volume ratio"
    )
    for cls in sorted(rows, key=lambda c: -rows[c][""]["n"]):
        row = rows[cls][""]
        print(
            f"{cls:<28} {row['n']:>6} {row['vol 1%']:>7} {row['vol 0.01%']:>8} {row['0.01 ft']:>8} {row['0.001 ft']:>8}"
            f" {row['ref on ours']:>11}  {statistics.median(ratios[cls]):.4f}"
        )
        types = sorted((k for k in rows[cls] if k), key=lambda k: -rows[cls][k]["n"])
        for key in types[:6]:
            t = rows[cls][key]
            print(
                f"    {t['n']:>5} {t['vol 1%']:>7} {t['vol 0.01%']:>8} {t['0.01 ft']:>8} {t['0.001 ft']:>8}"
                f" {t['ref on ours']:>11}  {key}"
            )
    scored.sort(reverse=True)
    for far, tag, cls, object_type, ratio, to_ours, to_ref in scored[:15]:
        print(
            f"  {tag} {cls} '{object_type}': volume ratio {ratio:.4f}, Revit's vertices to rvt-rs's surface"
            f" {to_ours:.4g} ft, rvt-rs's to Revit's {to_ref:.4g} ft"
        )
    # Bodies whose surfaces agree but whose enclosed volumes do not, which
    # the distance ranking above does not reach.
    outliers = sorted((abs(s[4] - 1), s) for s in scored if abs(s[4] - 1) > 0.01)
    for _, (far, tag, cls, object_type, ratio, to_ours, to_ref) in outliers[-15:]:
        print(
            f"  volume {tag} {cls} '{object_type}': volume ratio {ratio:.4f},"
            f" farthest vertex {far:.4g} ft"
        )
        for side, triangles in (("rvt-rs", ours[tag][2]), ("Revit", ref[tag][2])):
            shells = shell_volumes(triangles)
            listed = " ".join(f"{v:.4f}" for v in shells[:8])
            print(f"    {side} shells {len(shells)}, signed volumes (cubic ft): {listed}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

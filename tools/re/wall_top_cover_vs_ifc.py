#!/usr/bin/env python3
"""Where a floor or roof covers a wall's plan, and what Revit's wall top does
along its length (#1067, RE-267).

RE-258 compared each wall's lowest and highest point with Revit's. A floor
that covers only part of a wall's length cannot move the highest point if the
rest of the wall stays at its full height, and the scorer then reads the wall
as left whole. This scorer reads the whole length.

For each `IfcWall` both files hold under one `Tag` it meshes both bodies with
IfcOpenShell (the Body representation only, pieces under one Tag together),
maps Revit's into rvt-rs's frame with `wall_heights_vs_ifc.py`'s turn, shift
and height offset, and takes stations every 0.25 ft along the long axis of the
wall's plan (at most 400). At each station:

- the highest point of rvt-rs's wall and of Revit's, and how many distinct
  heights Revit's body has there (more than 2 means a slot or a step);
- for each floor, roof or footing whose body lies in the wall's height range
  and overlaps its plan, the underside there in rvt-rs's body and in Revit's.

Output, one record per line (runs are `from-to:height` in feet along the
wall from its start, `-` where the body is not there):

  walls <n> in both ...                    the counts and the fit
  wall tag:<Tag> length <L> thickness <T> area rvt-rs <A> Revit <A>
       differ <A> low/high rvt-rs <z z> Revit <z z>
       plan rvt-rs <x0 x1 y0 y1> Revit <x0 x1 y0 y1> stations <n>
  top tag:<Tag> rvt-rs <runs> | Revit <runs> | hits <max distinct heights>
  levels tag:<Tag> rvt-rs <z=area[x0 x1 y0 y1] ...> | Revit <...>
                                          the horizontal faces in the upper half
                                          of the body, highest first
  pair tag:<Tag> floor:<Tag> <entity> rvt-rs covers <fraction> gap <ft2>
       Revit covers <fraction> gap <ft2> | rvt-rs under <runs> | Revit under <runs>
  level tag:<Tag> floor:<Tag> rvt-rs <lowest highest> | Revit <...>
                                          the lowest and highest underside of the
                                          floor over three rows of stations (the
                                          centre line and 0.45 thicknesses off it
                                          on each side): equal where it is level
                                          across the wall
  gap tag:<Tag> floor:<Tag> rvt-rs <x0 x1 y0 y1 pieces n> | Revit <...>
                                          the extent of the wall's plan outside
                                          the floor's

`covers` is the share of the wall's plan area that lies inside the floor's
plan; `gap` the area of the wall's plan outside it.

Usage:

    python3 tools/re/wall_top_cover_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>

Needs IfcOpenShell, numpy and shapely (the versions tools/ci/requirements-ifc.txt pins).
"""

import math
import statistics
import sys
import warnings

import ifcopenshell
import ifcopenshell.geom
import numpy as np
import shapely

import wall_heights_vs_ifc as base

STEP = 0.25
MAX_STATIONS = 400
MAX_RUNS = 24
TOLERANCE = base.TOLERANCE
KINDS = base.CUTTERS


def bodies(path, kinds):
    """Tag -> an (N, 3, 3) array of the triangles of every product of `kinds`
    with a Body representation, in feet, the pieces under one Tag together,
    and Tag -> the entity."""
    f = ifcopenshell.open(path)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    parts, entities = {}, {}
    for kind in kinds:
        for product in f.by_type(kind):
            if not product.Tag or product.Representation is None:
                continue
            geometry = base.body_geometry(settings, product, path)
            if geometry is None or not len(geometry.verts):
                continue
            points = np.asarray(geometry.verts, dtype=float).reshape(-1, 3) / base.FOOT
            faces = np.asarray(geometry.faces, dtype=int).reshape(-1, 3)
            parts.setdefault(str(product.Tag), []).append(points[faces])
            entities[str(product.Tag)] = product.is_a()
    return {tag: np.concatenate(found) for tag, found in parts.items()}, entities


def middle(triangles):
    xs, ys = triangles[:, :, 0], triangles[:, :, 1]
    return np.array([(xs.min() + xs.max()) / 2, (ys.min() + ys.max()) / 2])


def to_ours(triangles, offset, fit):
    """Revit's triangles in rvt-rs's frame: the inverse of the turn and shift
    that wall_heights_vs_ifc.py fits, and the height offset taken off."""
    ours_middle, revit_middle, turn = fit
    cos_t, sin_t = math.cos(turn), math.sin(turn)
    x = triangles[:, :, 0] - revit_middle[0]
    y = triangles[:, :, 1] - revit_middle[1]
    out = triangles.copy()
    out[:, :, 0] = ours_middle[0] + cos_t * x + sin_t * y
    out[:, :, 1] = ours_middle[1] - sin_t * x + cos_t * y
    out[:, :, 2] = triangles[:, :, 2] - offset
    return out


def footprint(triangles):
    """The union of the triangles seen from above, or None."""
    polygons = shapely.polygons(shapely.linearrings(triangles[:, :, :2]))
    polygons = polygons[shapely.area(polygons) > 1e-9]
    if not len(polygons):
        return None
    try:
        return shapely.union_all(polygons)
    except Exception:
        return shapely.union_all(shapely.buffer(polygons, 1e-6))


def box(triangles):
    return (
        float(triangles[:, :, 0].min()),
        float(triangles[:, :, 0].max()),
        float(triangles[:, :, 1].min()),
        float(triangles[:, :, 1].max()),
        float(triangles[:, :, 2].min()),
        float(triangles[:, :, 2].max()),
    )


def heights_at(triangles, xs, ys):
    """For each station, the lowest and highest height at which the vertical
    line through it meets `triangles` (NaN where it meets none), and how many
    distinct heights (to 0.001 ft) it meets."""
    count = len(xs)
    nan = np.full(count, np.nan)
    if not len(triangles):
        return nan, nan.copy(), np.zeros(count, dtype=int)
    near = (
        (triangles[:, :, 0].max(axis=1) >= xs.min() - 1e-6)
        & (triangles[:, :, 0].min(axis=1) <= xs.max() + 1e-6)
        & (triangles[:, :, 1].max(axis=1) >= ys.min() - 1e-6)
        & (triangles[:, :, 1].min(axis=1) <= ys.max() + 1e-6)
    )
    triangles = triangles[near]
    if not len(triangles):
        return nan, nan.copy(), np.zeros(count, dtype=int)
    ax, ay, az = triangles[:, 0, 0], triangles[:, 0, 1], triangles[:, 0, 2]
    bx, by, bz = triangles[:, 1, 0], triangles[:, 1, 1], triangles[:, 1, 2]
    cx, cy, cz = triangles[:, 2, 0], triangles[:, 2, 1], triangles[:, 2, 2]
    det = (by - cy) * (ax - cx) + (cx - bx) * (ay - cy)
    usable = np.abs(det) > 1e-12
    x, y = xs[:, None], ys[:, None]
    with np.errstate(divide="ignore", invalid="ignore"):
        l1 = ((by - cy) * (x - cx) + (cx - bx) * (y - cy)) / det
        l2 = ((cy - ay) * (x - cx) + (ax - cx) * (y - cy)) / det
    l3 = 1 - l1 - l2
    hit = usable & (np.minimum(np.minimum(l1, l2), l3) >= -1e-9)
    z = np.where(hit, l1 * az + l2 * bz + l3 * cz, np.nan)
    any_hit = hit.any(axis=1)
    low = np.where(any_hit, np.nanmin(np.where(hit, z, np.inf), axis=1), np.nan)
    high = np.where(any_hit, np.nanmax(np.where(hit, z, -np.inf), axis=1), np.nan)
    rounded = np.sort(np.where(hit, np.round(z, 3), np.inf), axis=1)
    finite = np.isfinite(rounded)
    with np.errstate(invalid="ignore"):
        steps = np.diff(rounded, axis=1) > 0
    distinct = finite[:, :1].astype(int).sum(axis=1) + (steps & finite[:, 1:]).sum(axis=1)
    return low, high, distinct


def level_range(triangles, xs, ys, direction, thickness):
    """The lowest and highest underside of `triangles` over three rows of
    stations: the centre line and the two 0.45 thicknesses off it, or None
    where they meet none."""
    normal = np.array([-direction[1], direction[0]])
    found = []
    for share in (-0.45, 0.0, 0.45):
        low = heights_at(triangles, xs + normal[0] * thickness * share, ys + normal[1] * thickness * share)[0]
        found.append(low[np.isfinite(low)])
    found = np.concatenate(found)
    if not len(found):
        return None
    return float(found.min()), float(found.max())


def fmt_range(found):
    return "-" if found is None else f"{found[0]:.4f} {found[1]:.4f}"


def runs(values, cell, length):
    """`from-to:height` runs of equal heights (to 0.001 ft) along the wall."""
    keys = [None if math.isnan(v) else round(float(v), 3) for v in values]
    out = []
    start = 0
    for i in range(1, len(keys) + 1):
        if i == len(keys) or keys[i] != keys[start]:
            label = "-" if keys[start] is None else f"{float(values[start]):.4f}"
            out.append((start * cell, min(length, i * cell), label))
            start = i
    if len(out) > MAX_RUNS:
        seen = [float(v) for v in values if not math.isnan(v)]
        span = f"{min(seen):.4f}..{max(seen):.4f}" if seen else "-"
        return f"varies {span} in {len(out)} runs"
    return ",".join(f"{a:.2f}-{b:.2f}:{label}" for a, b, label in out)


def stations(plan):
    """Station points every STEP ft along the long axis of `plan`'s minimum
    rotated rectangle: (xs, ys, cell, length, thickness, direction), or
    None."""
    rectangle = shapely.minimum_rotated_rectangle(plan)
    if rectangle.geom_type != "Polygon":
        return None
    corners = np.array(rectangle.exterior.coords)[:4]
    first, second = corners[1] - corners[0], corners[2] - corners[1]
    long_edge, short_edge = (first, second) if np.hypot(*first) >= np.hypot(*second) else (second, first)
    length, thickness = float(np.hypot(*long_edge)), float(np.hypot(*short_edge))
    if length <= 1e-6:
        return None
    direction = long_edge / length
    count = min(MAX_STATIONS, max(1, math.ceil(length / STEP)))
    cell = length / count
    along = (np.arange(count) + 0.5) * cell
    start = corners.mean(axis=0) - direction * length / 2
    return start[0] + direction[0] * along, start[1] + direction[1] * along, cell, length, thickness, direction


def cover(wall_plan, floor_plan):
    """(share of `wall_plan` inside `floor_plan`, area of it outside, the part
    of it outside)."""
    if wall_plan is None or floor_plan is None:
        return None
    inside = wall_plan.intersection(floor_plan).area
    return inside / wall_plan.area, wall_plan.area - inside, wall_plan.difference(floor_plan)


def fmt_gap(found):
    """The extent and piece count of the part of the wall outside the floor."""
    if found is None or found[2].is_empty or found[1] <= 1e-6:
        return "-"
    pieces = len(found[2].geoms) if hasattr(found[2], "geoms") else 1
    x0, y0, x1, y1 = found[2].bounds
    return f"{x0:.4f} {x1:.4f} {y0:.4f} {y1:.4f} pieces {pieces}"


def levels(triangles):
    """The horizontal faces in the upper half of `triangles`' height, by
    height (to 0.001 ft): `z=area[x0 x1 y0 y1]`, highest first. A triangle
    is horizontal when its three corners lie within 0.001 ft of each other in
    height, so a shallow slope is not read as a level face."""
    edge_a = triangles[:, 1] - triangles[:, 0]
    edge_b = triangles[:, 2] - triangles[:, 0]
    normal = np.cross(edge_a, edge_b)
    size = np.linalg.norm(normal, axis=1)
    spread = triangles[:, :, 2].max(axis=1) - triangles[:, :, 2].min(axis=1)
    flat = (size > 1e-12) & (spread <= 0.001)
    middle_height = (triangles[:, :, 2].min() + triangles[:, :, 2].max()) / 2
    heights = triangles[:, :, 2].mean(axis=1)
    found = {}
    for index in np.nonzero(flat & (heights > middle_height))[0]:
        found.setdefault(round(float(heights[index]), 3), []).append(triangles[index])
    out = []
    for height in sorted(found, reverse=True):
        plan = footprint(np.array(found[height]))
        if plan is None:
            continue
        x0, y0, x1, y1 = plan.bounds
        out.append(f"{height:.3f}={plan.area:.4f}[{x0:.4f} {x1:.4f} {y0:.4f} {y1:.4f}]")
    return " ".join(out) if out else "-"


def fmt_cover(found):
    return "- gap -" if found is None else f"{found[0]:.4f} gap {found[1]:.4f}"


def main():
    args = sys.argv[1:]
    if len(args) != 2:
        print(__doc__)
        return 2
    warnings.simplefilter("ignore", RuntimeWarning)
    ours_walls, _ = bodies(args[0], ("IfcWall",))
    revit_walls, _ = bodies(args[1], ("IfcWall",))
    both = sorted(set(ours_walls) & set(revit_walls))
    if not both:
        print("no wall in both files")
        return 0
    offset = statistics.median(float(revit_walls[t][:, :, 2].min() - ours_walls[t][:, :, 2].min()) for t in both)
    a = np.array([middle(ours_walls[t]) for t in both])
    b = np.array([middle(revit_walls[t]) for t in both])
    ours_middle, revit_middle = a.mean(axis=0), b.mean(axis=0)
    da, db = a - ours_middle, b - revit_middle
    turn = math.atan2(float((da[:, 0] * db[:, 1] - da[:, 1] * db[:, 0]).sum()), float((da * db).sum()))
    fit = (ours_middle, revit_middle, turn)
    print(
        f"walls {len(both)} in both files (rvt-rs {len(ours_walls)}, Revit {len(revit_walls)}); "
        f"offset {offset:.6f} ft; plan turn {math.degrees(turn):.6f} degrees"
    )
    ours_slabs, kinds = bodies(args[0], KINDS)
    revit_found, revit_kinds = bodies(args[1], KINDS)
    revit_slabs = {tag: to_ours(t, offset, fit) for tag, t in revit_found.items()}
    kinds = {**revit_kinds, **kinds}
    print(f"floors, roofs and footings: rvt-rs {len(ours_slabs)}, Revit {len(revit_slabs)}")
    slab_boxes = {
        tag: (box(ours_slabs[tag]) if tag in ours_slabs else None, box(revit_slabs[tag]) if tag in revit_slabs else None)
        for tag in set(ours_slabs) | set(revit_slabs)
    }
    plans = {}

    def slab_plan(source, tag, table):
        if (source, tag) not in plans:
            plans[(source, tag)] = footprint(table[tag]) if tag in table else None
        return plans[(source, tag)]

    for tag in both:
        ours_body = ours_walls[tag]
        revit_body = to_ours(revit_walls[tag], offset, fit)
        ours_plan, revit_plan = footprint(ours_body), footprint(revit_body)
        if ours_plan is None or revit_plan is None:
            print(f"wall tag:{tag} plan not meshed")
            continue
        along = stations(ours_plan)
        if along is None:
            print(f"wall tag:{tag} no long axis")
            continue
        xs, ys, cell, length, thickness, direction = along
        ours_box, revit_box = box(ours_body), box(revit_body)
        print(
            f"wall tag:{tag} length {length:.4f} thickness {thickness:.4f} area rvt-rs {ours_plan.area:.4f} "
            f"Revit {revit_plan.area:.4f} differ {ours_plan.symmetric_difference(revit_plan).area:.4f} "
            f"low/high rvt-rs {ours_box[4]:.4f} {ours_box[5]:.4f} Revit {revit_box[4]:.4f} {revit_box[5]:.4f} "
            f"plan rvt-rs {ours_box[0]:.4f} {ours_box[1]:.4f} {ours_box[2]:.4f} {ours_box[3]:.4f} "
            f"Revit {revit_box[0]:.4f} {revit_box[1]:.4f} {revit_box[2]:.4f} {revit_box[3]:.4f} stations {len(xs)}"
        )
        print(f"levels tag:{tag} rvt-rs {levels(ours_body)} | Revit {levels(revit_body)}")
        _, ours_high, _ = heights_at(ours_body, xs, ys)
        _, revit_high, hits = heights_at(revit_body, xs, ys)
        print(
            f"top tag:{tag} rvt-rs {runs(ours_high, cell, length)} | Revit {runs(revit_high, cell, length)} "
            f"| hits {int(hits.max()) if len(hits) else 0}"
        )
        low, high = ours_box[4], ours_box[5]
        reach = (min(ours_box[0], revit_box[0]) - 0.1, max(ours_box[1], revit_box[1]) + 0.1,
                 min(ours_box[2], revit_box[2]) - 0.1, max(ours_box[3], revit_box[3]) + 0.1)
        for slab in sorted(slab_boxes):
            seen = [b for b in slab_boxes[slab] if b is not None]
            if not any(
                b[1] >= reach[0] and b[0] <= reach[1] and b[3] >= reach[2] and b[2] <= reach[3]
                and b[5] > low + TOLERANCE and b[4] < high - TOLERANCE
                for b in seen
            ):
                continue
            ours_under = heights_at(ours_slabs[slab], xs, ys)[0] if slab in ours_slabs else np.full(len(xs), np.nan)
            revit_under = heights_at(revit_slabs[slab], xs, ys)[0] if slab in revit_slabs else np.full(len(xs), np.nan)
            ours_cover = cover(ours_plan, slab_plan("ours", slab, ours_slabs))
            revit_cover = cover(revit_plan, slab_plan("revit", slab, revit_slabs))
            touched = np.isfinite(ours_under).any() or np.isfinite(revit_under).any()
            reaches = any(c is not None and c[0] > 1e-6 for c in (ours_cover, revit_cover))
            if not touched and not reaches:
                continue
            print(
                f"pair tag:{tag} floor:{slab} {kinds.get(slab, '-')} rvt-rs covers {fmt_cover(ours_cover)} "
                f"Revit covers {fmt_cover(revit_cover)} | rvt-rs under {runs(ours_under, cell, length)} "
                f"| Revit under {runs(revit_under, cell, length)}"
            )
            print(
                f"level tag:{tag} floor:{slab} rvt-rs {fmt_range(level_range(ours_slabs[slab], xs, ys, direction, thickness)) if slab in ours_slabs else '-'} "
                f"| Revit {fmt_range(level_range(revit_slabs[slab], xs, ys, direction, thickness)) if slab in revit_slabs else '-'}"
            )
            if fmt_gap(ours_cover) != "-" or fmt_gap(revit_cover) != "-":
                print(f"gap tag:{tag} floor:{slab} rvt-rs {fmt_gap(ours_cover)} | Revit {fmt_gap(revit_cover)}")
    unmeshed = {}
    for (path, _), (kind, reason) in base.UNMESHED.items():
        key = ("rvt-rs" if path == args[0] else "Revit", kind, reason)
        unmeshed[key] = unmeshed.get(key, 0) + 1
    for (name, kind, reason), n in sorted(unmeshed.items()):
        print(f"left out of {name}'s file: {kind}, {reason}: {n}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

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
  plan tag:<Tag> points <n> equal <n> taller <n> lower <n> | form <...>
                                          the highest point of rvt-rs's body
                                          against Revit's at 5 rows of stations
                                          across the wall (0.4 and 0.2 thicknesses
                                          off the centre line on each side, and the
                                          centre line), at the points inside both
                                          plans: equal within 0.01 ft, rvt-rs's
                                          taller, rvt-rs's lower (RE-320; a strip
                                          of the wall beside the centre line that a
                                          floor covers cannot show on the centre
                                          line); Revit's body is shifted across the
                                          wall's thickness so that the two plans'
                                          faces lie midway (the fit of Revit's frame
                                          onto rvt-rs's leaves up to 0.02 ft); form
                                          is how each file draws the wall's Body
  faces tag:<Tag> top <z> rvt-rs <A> Revit <A> taller <A> lower <A> volume rvt-rs <V> Revit <V>
                                          the plan area of the horizontal faces at
                                          rvt-rs's highest point in each body, and,
                                          inside the part of the plan both files
                                          draw, the area where rvt-rs is at that
                                          height and Revit's body is not (taller)
                                          and where Revit's is and rvt-rs's is not
                                          (lower), in square feet; and each body's
                                          volume, cubic feet
  piece tag:<Tag> area <A> width <W> at <x> <y> inside Revit's plan <A> at Revit's full height <share> in other walls <share>
                                          for a wall rvt-rs steps: each piece of
                                          its plan at its highest point, its area,
                                          its least width, and the share of it
                                          inside Revit's plan where Revit's body is
                                          at that height too, and the share of it
                                          that lies inside the plan of another wall
                                          of rvt-rs's that shares more than 0.5 ft of
                                          its height
  summary ...                             walls equal on the centre line, and
                                          over the whole plan, and the points

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
import shapely.affinity

import wall_heights_vs_ifc as base

STEP = 0.25
MAX_STATIONS = 400
MAX_RUNS = 24
PLAN_ROWS = (-0.4, -0.2, 0.0, 0.2, 0.4)
PLAN_TOLERANCE = 0.01
CHUNK = 400
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


def forms(path):
    """Tag -> how the file draws the wall's Body: its representation type and
    the classes of its items (an extrusion's profile class after a slash)."""
    f = ifcopenshell.open(path)
    out = {}
    for product in f.by_type("IfcWall"):
        if not product.Tag or product.Representation is None:
            continue
        found = []
        for representation in product.Representation.Representations:
            if representation.RepresentationIdentifier != "Body":
                continue
            for item in representation.Items:
                name = item.is_a()
                if item.is_a("IfcExtrudedAreaSolid"):
                    name += "/" + item.SweptArea.is_a()
                found.append((representation.RepresentationType or "-", name))
        if found:
            counts = {}
            for _, name in found:
                counts[name] = counts.get(name, 0) + 1
            out[str(product.Tag)] = found[0][0] + " " + " ".join(f"{n}x{c}" for n, c in sorted(counts.items()))
    return out


def high_at(triangles, xs, ys):
    """The highest point at each of the points, in chunks."""
    parts = [heights_at(triangles, xs[i:i + CHUNK], ys[i:i + CHUNK])[1] for i in range(0, len(xs), CHUNK)]
    return np.concatenate(parts) if parts else np.zeros(0)


def plan_top(ours_body, revit_body, ours_plan, revit_plan, xs, ys, direction, thickness):
    """(points, equal, taller, lower): rvt-rs's highest point against Revit's
    over PLAN_ROWS rows of stations across the wall, at the points inside
    both plans where both bodies are, or None."""
    normal = np.array([-direction[1], direction[0]])
    px = np.concatenate([xs + normal[0] * thickness * row for row in PLAN_ROWS])
    py = np.concatenate([ys + normal[1] * thickness * row for row in PLAN_ROWS])
    inside = shapely.contains_xy(ours_plan, px, py) & shapely.contains_xy(revit_plan, px, py)
    px, py = px[inside], py[inside]
    if not len(px):
        return None
    ours_high, revit_high = high_at(ours_body, px, py), high_at(revit_body, px, py)
    both = np.isfinite(ours_high) & np.isfinite(revit_high)
    if not both.any():
        return None
    gap = ours_high[both] - revit_high[both]
    return (
        int(both.sum()),
        int((np.abs(gap) <= PLAN_TOLERANCE).sum()),
        int((gap > PLAN_TOLERANCE).sum()),
        int((gap < -PLAN_TOLERANCE).sum()),
    )


def across_shift(ours_plan, revit_plan, direction):
    """The shift, across the wall, that puts Revit's plan midway between the
    same faces as rvt-rs's: the two plans' extents along the normal to the
    wall's long axis, centred on each other."""
    normal = np.array([-direction[1], direction[0]])

    def extent(plan):
        along = np.array(plan.convex_hull.exterior.coords) @ normal
        return float(along.min() + along.max()) / 2

    return (extent(ours_plan) - extent(revit_plan)) * normal


def shifted(triangles, shift):
    out = triangles.copy()
    out[:, :, 0] += shift[0]
    out[:, :, 1] += shift[1]
    return out


def volume(triangles):
    """The volume of the closed surface `triangles`, cubic feet."""
    corner_a, corner_b, corner_c = triangles[:, 0], triangles[:, 1], triangles[:, 2]
    return abs(float(np.einsum("ij,ij->i", corner_a, np.cross(corner_b, corner_c)).sum() / 6.0))


def top_faces(triangles, top):
    """The union, seen from above, of the upward horizontal faces of
    `triangles` within 0.01 ft of the height `top`, or an empty polygon."""
    corner_a = triangles[:, 1] - triangles[:, 0]
    corner_b = triangles[:, 2] - triangles[:, 0]
    normal = np.cross(corner_a, corner_b)
    size = np.linalg.norm(normal, axis=1)
    spread = triangles[:, :, 2].max(axis=1) - triangles[:, :, 2].min(axis=1)
    heights = triangles[:, :, 2].mean(axis=1)
    flat = (size > 1e-12) & (spread <= 0.001) & (normal[:, 2] > 0) & (np.abs(heights - top) <= 0.01)
    found = triangles[flat]
    plan = footprint(found) if len(found) else None
    return shapely.Polygon() if plan is None else plan


def pieces_of(plan):
    return list(plan.geoms) if hasattr(plan, "geoms") else [plan]


def least_width(polygon):
    rectangle = shapely.minimum_rotated_rectangle(polygon)
    if rectangle.geom_type != "Polygon":
        return 0.0
    corners = np.array(rectangle.exterior.coords)[:4]
    return float(min(np.hypot(*(corners[1] - corners[0])), np.hypot(*(corners[2] - corners[1]))))


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
    ours_forms, revit_forms = forms(args[0]), forms(args[1])
    wall_tags = list(both)
    wall_plans = [footprint(ours_walls[t]) for t in wall_tags]
    wall_index = shapely.STRtree([p if p is not None else shapely.Polygon() for p in wall_plans])
    wall_heights = [(float(ours_walls[t][:, :, 2].min()), float(ours_walls[t][:, :, 2].max())) for t in wall_tags]
    on_centre = on_plan = walls_planned = 0
    plan_points = [0, 0, 0, 0]
    tallers = lowers = 0.0

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
        ours_runs, revit_runs = runs(ours_high, cell, length), runs(revit_high, cell, length)
        print(
            f"top tag:{tag} rvt-rs {ours_runs} | Revit {revit_runs} "
            f"| hits {int(hits.max()) if len(hits) else 0}"
        )
        on_centre += ours_runs == revit_runs
        shift = across_shift(ours_plan, revit_plan, direction)
        revit_across = shifted(revit_body, shift)
        revit_plan_across = shapely.affinity.translate(revit_plan, shift[0], shift[1])
        planned = plan_top(ours_body, revit_across, ours_plan, revit_plan_across, xs, ys, direction, thickness)
        if planned is None:
            print(f"plan tag:{tag} points 0 | form rvt-rs {ours_forms.get(tag, '-')} | Revit {revit_forms.get(tag, '-')}")
        else:
            walls_planned += 1
            on_plan += planned[0] == planned[1]
            plan_points = [a + b for a, b in zip(plan_points, planned)]
            print(
                f"plan tag:{tag} points {planned[0]} equal {planned[1]} taller {planned[2]} lower {planned[3]} "
                f"| form rvt-rs {ours_forms.get(tag, '-')} | Revit {revit_forms.get(tag, '-')}"
            )
        top = ours_box[5]
        ours_full, revit_full = top_faces(ours_body, top), top_faces(revit_across, top)
        common = ours_plan.intersection(revit_plan_across)
        taller = ours_full.difference(revit_full).intersection(common).area
        lower = revit_full.difference(ours_full).intersection(common).area
        print(
            f"faces tag:{tag} top {top:.4f} rvt-rs {ours_full.area:.4f} Revit {revit_full.area:.4f} "
            f"taller {taller:.4f} lower {lower:.4f} volume rvt-rs {volume(ours_body):.4f} Revit {volume(revit_body):.4f}"
        )
        tallers += taller
        lowers += lower
        low, high = ours_box[4], ours_box[5]
        if ours_full.area < ours_plan.area - 1e-3:
            for piece in pieces_of(ours_full):
                inside = piece.intersection(common).area
                if inside > 1e-6:
                    others = [
                        wall_plans[i]
                        for i in wall_index.query(piece)
                        if wall_tags[i] != tag
                        and wall_plans[i] is not None
                        and min(high, wall_heights[i][1]) - max(low, wall_heights[i][0]) > 0.5
                    ]
                    beside = piece.intersection(shapely.union_all(others)).area / piece.area if others else 0.0
                    print(
                        f"piece tag:{tag} area {piece.area:.4f} width {least_width(piece):.4f} at {piece.centroid.x:.4f} {piece.centroid.y:.4f} "
                        f"inside Revit's plan {inside:.4f} at Revit's full height {piece.intersection(common).intersection(revit_full).area / inside:.4f} "
                        f"in other walls {beside:.4f}"
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
    print(
        f"summary walls {len(both)}; equal on the centre line {on_centre}; "
        f"equal over the plan {on_plan} of {walls_planned} compared; "
        f"points {plan_points[0]} equal {plan_points[1]} taller {plan_points[2]} lower {plan_points[3]}; "
        f"full-height area taller {tallers:.4f} lower {lowers:.4f} ft2"
    )
    unmeshed = {}
    for (path, _), (kind, reason) in base.UNMESHED.items():
        key = ("rvt-rs" if path == args[0] else "Revit", kind, reason)
        unmeshed[key] = unmeshed.get(key, 0) + 1
    for (name, kind, reason), n in sorted(unmeshed.items()):
        print(f"left out of {name}'s file: {kind}, {reason}: {n}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""What Revit's export does with the part of a wall's plan that a floor, roof
or footing leaves uncovered (#1067, RE-320).

RE-320 draws a wall's top at the underside of the element over the part of the
plan the element covers and at the wall's own height over the rest. The one
wall that rule gets wrong on the BIM4LCA office as built (3004978) is a wall
whose roof covers all of its plan but a stub 0.0853 ft long at its end, inside
the wall it abuts: Revit's body is cut at the roof's underside there too, over
the stub, while Revit keeps most other pieces outside a floor at the wall's own
height (RE-320 section 4). A rule for the stub that rests on one wall is not
enough, so this scorer reads Revit's export alone, over every wall, to see
what separates the pieces Revit cuts from the ones it keeps.

For each `IfcWall` both files hold under one `Tag` it takes Revit's bodies
(wall_top_cover_vs_ifc.py's pairing and fit, the pieces under one Tag
together), and:

- the wall's own height: the higher of the two files' highest points;
- the elements that cut it: the floors, roofs and footings of Revit's export
  whose lowest point lies inside the wall's height, whose plan meets the
  wall's, and under whose plan Revit's wall has horizontal faces at that
  lowest point (over 0.001 ft2);
- the pieces of Revit's plan of the wall outside the plans of all of them,
  over 1e-4 ft2.

Output, one record per line (lengths in feet, areas in square feet):

  census walls <n> in both files; with an element cutting part of the plan <m>
  top tag:<Tag> rvt-rs <z> Revit <z>
                                 the highest point of the wall in each file, for
                                 a wall that has a piece
  piece tag:<Tag> floors <Tags> area <A> width <W> along <s0> <s1> of <L>
       across <t0> <t1> of <T> in other walls <share> kept <share>
       | <class> | form <Revit's Body> | joins <...>
                                 a piece: its extent along the wall's long axis
                                 from its start and across it from one face, the
                                 share of it inside the plan of another of
                                 Revit's walls that shares more than 0.5 ft of the
                                 wall's height, and the share of it at which
                                 Revit's body is at the wall's own height
                                 (`kept`; 0 where it is cut)
  axis tag:<Tag> <s0> <s1> inside the elements <share>
                                 Revit's Axis of a wall that has a piece: where it
                                 starts and ends along the same axis and the share
                                 of its length inside the plans of the elements
  census <class> along <span|end|inner> across <span|end|inner> embedded <0|1> small <0|1>: <n>
                                 pieces by the class of Revit's top there (kept
                                 at 0.99 or more, cut at 0.01 or less, mixed);
                                 how the piece lies along the wall's plan (span
                                 reaches both ends, end one of them, inner
                                 neither) and across it (span reaches both faces,
                                 end one face, inner neither); whether 0.99 or
                                 more of it lies inside other walls; and whether
                                 it has under 0.05 ft2
  census <class> form <Revit's Body>: <n>

Usage:

    python3 tools/re/wall_outside_floor_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>

Needs IfcOpenShell, numpy and shapely (the versions tools/ci/requirements-ifc.txt pins).
"""

import collections
import math
import statistics
import sys
import warnings

import ifcopenshell
import ifcopenshell.util.placement
import ifcopenshell.util.unit
import numpy as np
import shapely

import wall_heights_vs_ifc as base
import wall_top_cover_vs_ifc as top

MIN_AREA = 1e-4
EDGE = 0.001
KEPT = 0.99
CUT = 0.01
SMALL = 0.05
SHARED_HEIGHT = 0.5


def revit_axes(path, offset, fit):
    """Tag -> the two ends (x, y) of the wall's Axis in rvt-rs's frame, for the
    walls whose Axis is a polyline. Empty where the file has none."""
    out = {}
    try:
        f = ifcopenshell.open(path)
        scale = ifcopenshell.util.unit.calculate_unit_scale(f)
        ours_middle, revit_middle, turn = fit
        cos_t, sin_t = math.cos(turn), math.sin(turn)
        for wall in f.by_type("IfcWall"):
            if not wall.Tag or wall.Representation is None or wall.ObjectPlacement is None:
                continue
            axis = next((r for r in wall.Representation.Representations if r.RepresentationIdentifier == "Axis"), None)
            if axis is None or not axis.Items or not axis.Items[0].is_a("IfcPolyline"):
                continue
            points = axis.Items[0].Points
            if len(points) < 2:
                continue
            matrix = ifcopenshell.util.placement.get_local_placement(wall.ObjectPlacement)
            ends = []
            for p in (points[0], points[-1]):
                c = list(p.Coordinates) + [0.0] * (3 - len(p.Coordinates))
                world = matrix @ np.array([c[0], c[1], c[2], 1.0])
                x = world[0] * scale / base.FOOT - revit_middle[0]
                y = world[1] * scale / base.FOOT - revit_middle[1]
                ends.append((ours_middle[0] + cos_t * x + sin_t * y, ours_middle[1] - sin_t * x + cos_t * y))
            out[str(wall.Tag)] = ends
    except Exception as error:  # the Axis is evidence for a side note, never for the census
        print(f"axes not read: {error}")
    return out


def joins(path):
    """Tag -> how each wall meets the others in Revit's export, as
    `<its end>-><partner Tag>/<partner's end>`, `ATPATH` meaning along the
    partner's length."""
    out = {}
    f = ifcopenshell.open(path)
    for rel in f.by_type("IfcRelConnectsPathElements"):
        a, b = rel.RelatingElement, rel.RelatedElement
        if not (a and b and a.Tag and b.Tag):
            continue
        out.setdefault(str(a.Tag), []).append(f"{rel.RelatingConnectionType}->{b.Tag}/{rel.RelatedConnectionType}")
        out.setdefault(str(b.Tag), []).append(f"{rel.RelatedConnectionType}->{a.Tag}/{rel.RelatingConnectionType}")
    return out


def polygons_of(shape):
    """The polygons of a shapely geometry, whatever it is."""
    if shape is None or shape.is_empty:
        return []
    if shape.geom_type == "Polygon":
        return [shape]
    return [g for part in getattr(shape, "geoms", []) for g in polygons_of(part)]


def reach(low, high, size):
    """How [low, high] lies in [0, size]: `span` if it reaches both ends, `end`
    if it reaches one, `inner` if neither."""
    first, last = low <= EDGE, high >= size - EDGE
    return "span" if first and last else ("end" if first or last else "inner")


def class_of(share):
    return "kept" if share >= KEPT else ("cut" if share <= CUT else "mixed")


def census_wall(tag, plan, high, floors, other_plans, form, linked, axis, tally, shapes):
    """The `piece` and `axis` lines of one wall, and its pieces counted in
    `tally`. `floors` is Tag -> (plan, box) of the elements that cut it,
    `other_plans` the plans of Revit's other walls that share its height, and
    `shapes(z)` the upward horizontal faces of Revit's body at height `z`."""
    covered = shapely.union_all([floor_plan for floor_plan, _ in floors.values()])
    pieces = [p for p in polygons_of(plan.difference(covered)) if p.area > MIN_AREA]
    if not pieces:
        return []
    along = top.stations(plan)
    if along is None:
        return []
    _, _, _, length, thickness, direction = along
    rectangle = shapely.minimum_rotated_rectangle(plan)
    center = np.array(rectangle.centroid.coords[0])
    normal = np.array([-direction[1], direction[0]])
    origin_s = float(center @ direction) - length / 2
    origin_t = float(center @ normal) - thickness / 2
    full = shapes(high)
    beside = shapely.union_all(other_plans) if other_plans else None
    lines = []
    for piece in pieces:
        points = np.array(piece.exterior.coords)
        s, t = points @ direction - origin_s, points @ normal - origin_t
        kept = 0.0 if full.is_empty else piece.intersection(full).area / piece.area
        inside = 0.0 if beside is None else piece.intersection(beside).area / piece.area
        lengthwise = reach(float(s.min()), float(s.max()), length)
        crosswise = reach(float(t.min()), float(t.max()), thickness)
        embedded = int(inside >= KEPT)
        small = int(piece.area < SMALL)
        label = class_of(kept)
        tally[(label, lengthwise, crosswise, embedded, small)] += 1
        tally[(label, form)] += 1
        lines.append(
            f"piece tag:{tag} floors {','.join(sorted(floors))} area {piece.area:.4f} width {top.least_width(piece):.4f} "
            f"along {s.min():.4f} {s.max():.4f} of {length:.4f} across {t.min():.4f} {t.max():.4f} of {thickness:.4f} "
            f"in other walls {inside:.4f} kept {kept:.4f} | {label} | form {form} | joins {' '.join(linked) if linked else '-'}"
        )
    if axis is not None:
        line = shapely.LineString(axis)
        share = line.intersection(covered).length / line.length if line.length > 1e-9 else 0.0
        a0, a1 = (float(np.array(p) @ direction - origin_s) for p in axis)
        lines.append(f"axis tag:{tag} {a0:.4f} {a1:.4f} inside the elements {share:.4f}")
    return lines


def main():
    args = sys.argv[1:]
    if len(args) != 2:
        print(__doc__)
        return 2
    warnings.simplefilter("ignore", RuntimeWarning)
    ours_walls, _ = top.bodies(args[0], ("IfcWall",))
    revit_found, _ = top.bodies(args[1], ("IfcWall",))
    both = sorted(set(ours_walls) & set(revit_found))
    if not both:
        print("no wall in both files")
        return 0
    offset = statistics.median(float(revit_found[t][:, :, 2].min() - ours_walls[t][:, :, 2].min()) for t in both)
    a = np.array([top.middle(ours_walls[t]) for t in both])
    b = np.array([top.middle(revit_found[t]) for t in both])
    ours_middle, revit_middle = a.mean(axis=0), b.mean(axis=0)
    da, db = a - ours_middle, b - revit_middle
    turn = math.atan2(float((da[:, 0] * db[:, 1] - da[:, 1] * db[:, 0]).sum()), float((da * db).sum()))
    fit = (ours_middle, revit_middle, turn)
    revit_walls = {t: top.to_ours(revit_found[t], offset, fit) for t in both}
    slab_found, _ = top.bodies(args[1], top.KINDS)
    slabs = {t: top.to_ours(v, offset, fit) for t, v in slab_found.items()}
    slab_plans = {t: top.footprint(v) for t, v in slabs.items()}
    slab_boxes = {t: top.box(v) for t, v in slabs.items()}
    wall_plans = {t: top.footprint(revit_walls[t]) for t in both}
    wall_boxes = {t: top.box(revit_walls[t]) for t in both}
    tags = [t for t in both if wall_plans[t] is not None]
    index = shapely.STRtree([wall_plans[t] for t in tags])
    forms = top.forms(args[1])
    links = joins(args[1])
    axes = revit_axes(args[1], offset, fit)
    tally = collections.Counter()
    cut_walls = 0
    lines = []
    for tag in tags:
        body, plan = revit_walls[tag], wall_plans[tag]
        box = wall_boxes[tag]
        low = box[4]
        high = max(box[5], float(ours_walls[tag][:, :, 2].max()))
        floors = {}
        for slab, slab_box in slab_boxes.items():
            slab_plan = slab_plans[slab]
            if not (slab_box[5] > low + base.TOLERANCE and slab_box[4] < high - base.TOLERANCE):
                continue
            if slab_plan is None or not slab_plan.intersects(plan):
                continue
            faces = top.top_faces(body, slab_box[4])
            if not faces.is_empty and faces.intersection(slab_plan).area > 1e-3:
                floors[slab] = (slab_plan, slab_box)
        if not floors:
            continue
        cut_walls += 1
        cache = {}

        def shapes(z, body=body, cache=cache):
            if z not in cache:
                cache[z] = top.top_faces(body, z)
            return cache[z]

        others = [
            wall_plans[tags[i]]
            for i in index.query(plan.buffer(0.01))
            if tags[i] != tag and min(high, wall_boxes[tags[i]][5]) - max(low, wall_boxes[tags[i]][4]) > SHARED_HEIGHT
        ]
        found = census_wall(
            tag, plan, high, floors, others, forms.get(tag, "-"), links.get(tag), axes.get(tag), tally, shapes
        )
        if found:
            lines.append(f"top tag:{tag} rvt-rs {float(ours_walls[tag][:, :, 2].max()):.4f} Revit {box[5]:.4f}")
            lines.extend(found)
    print(f"census walls {len(both)} in both files; with an element cutting part of the plan {cut_walls}")
    for line in lines:
        print(line)
    pieces = sum(n for key, n in tally.items() if len(key) == 5)
    print(f"census pieces {pieces}")
    for key in sorted(k for k in tally if len(k) == 5):
        label, lengthwise, crosswise, embedded, small = key
        print(
            f"census {label} along {lengthwise} across {crosswise} embedded {embedded} small {small}: {tally[key]}"
        )
    for key in sorted(k for k in tally if len(k) == 2):
        print(f"census {key[0]} form {key[1]}: {tally[key]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

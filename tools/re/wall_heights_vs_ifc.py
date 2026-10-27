#!/usr/bin/env python3
"""Compare how high each wall of an rvt-rs IFC stands with Revit's own IFC
export of the same file (#1067, RE-257).

For each `IfcWall` both files hold under one `Tag` (the Revit ElementId), it
meshes both bodies with IfcOpenShell in world coordinates and takes their
lowest and highest points. Revit's export is in shared coordinates, so the
two files' heights differ by one offset: the median of the walls' base
differences. It then counts the walls whose base and top are Revit's
within 0.001 ft, those where only one of them is, and those where neither
is, and gives each file's storey for the walls that miss. Plan positions
are `wall_bodies_vs_ifc.py`'s. It then tests one explanation of the misses:
that Revit cuts a wall where a floor, roof or footing whose plan box
overlaps the wall's crosses its top or its base, and counts for each end
whether that cut gives Revit's.

Usage:

    python3 tools/re/wall_heights_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>

It lists at most 150 of the walls that miss.

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import collections
import math
import statistics
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element

FOOT = 0.3048
TOLERANCE = 0.001


CUTTERS = ("IfcSlab", "IfcRoof", "IfcFooting")


FOOTPRINTS = {}


def extents(f, entity):
    """(lowest z, highest z, x min, x max, y min, y max) of a product's body
    in feet, or None. Its triangles in plan go to FOOTPRINTS by Tag."""
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    try:
        geometry = ifcopenshell.geom.create_shape(settings, entity).geometry
        verts, faces = geometry.verts, geometry.faces
    except Exception:
        return None
    if not verts:
        return None
    xs, ys, zs = verts[0::3], verts[1::3], verts[2::3]
    FOOTPRINTS[str(entity.Tag)] = [
        tuple(c / FOOT for k in faces[i : i + 3] for c in (xs[k], ys[k])) for i in range(0, len(faces), 3)
    ]
    return tuple(v / FOOT for v in (min(zs), max(zs), min(xs), max(xs), min(ys), max(ys)))


def covers(tag, x, y):
    """Whether the plan point (x, y) lies on the footprint of the body `tag`
    names: inside one of its triangles seen from above."""
    for ax, ay, bx, by, cx, cy in FOOTPRINTS.get(tag, ()):
        d1 = (x - bx) * (ay - by) - (ax - bx) * (y - by)
        d2 = (x - cx) * (by - cy) - (bx - cx) * (y - cy)
        d3 = (x - ax) * (cy - ay) - (cx - ax) * (y - ay)
        if not ((d1 < 0 or d2 < 0 or d3 < 0) and (d1 > 0 or d2 > 0 or d3 > 0)):
            if (ax - bx) * (cy - by) - (ay - by) * (cx - bx) != 0:
                return True
    return False


def cutters(path):
    """(extents, Tag) of every floor, roof and footing body."""
    f = ifcopenshell.open(path)
    out = []
    for kind in CUTTERS:
        for product in f.by_type(kind):
            if product.Representation is None:
                continue
            box = extents(f, product)
            if box is not None:
                out.append((box, str(product.Tag or "-")))
                body = sorted(
                    {r.RepresentationType or "-" for r in product.Representation.Representations if r.RepresentationIdentifier == "Body"}
                )
                print(f"cutter tag:{product.Tag} {kind} {'/'.join(body)} " + " ".join(f"{v:.4f}" for v in box))
    return out


def crossing(wall, boxes):
    """The (Tag, end, its height) of each floor, roof or footing whose plan
    box overlaps `wall` and that crosses its top (at its underside) or its
    base (at its top)."""
    low, high, x0, x1, y0, y1 = wall
    out = []
    for (c_low, c_high, cx0, cx1, cy0, cy1), tag in boxes:
        if min(x1, cx1) - max(x0, cx0) <= TOLERANCE or min(y1, cy1) - max(y0, cy0) <= TOLERANCE:
            continue
        if low + TOLERANCE < c_low < high - TOLERANCE and c_high >= high - TOLERANCE:
            out.append((tag, "top", c_low))
        if low + TOLERANCE < c_high < high - TOLERANCE and c_low <= low + TOLERANCE:
            out.append((tag, "base", c_high))
    return out


def predicted(wall, boxes):
    """The base and top Revit would give `wall` if each floor, roof or
    footing whose plan box overlaps it cut it where it crosses its top or
    base: (base, top) in feet."""
    low, high, x0, x1, y0, y1 = wall
    base, top = low, high
    for (c_low, c_high, cx0, cx1, cy0, cy1), _ in boxes:
        if min(x1, cx1) - max(x0, cx0) <= TOLERANCE or min(y1, cy1) - max(y0, cy0) <= TOLERANCE:
            continue
        if low + TOLERANCE < c_low < high - TOLERANCE and c_high >= high - TOLERANCE:
            top = min(top, c_low)
        if low + TOLERANCE < c_high < high - TOLERANCE and c_low <= low + TOLERANCE:
            base = max(base, c_high)
    return base, top


def surfaces(path):
    """Tag -> the triangles (three (x, y, z) points, feet) of each floor,
    roof and footing body in `path`."""
    f = ifcopenshell.open(path)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    out = {}
    for kind in CUTTERS:
        for product in f.by_type(kind):
            if product.Representation is None or not product.Tag:
                continue
            try:
                geometry = ifcopenshell.geom.create_shape(settings, product).geometry
            except Exception:
                continue
            v, faces = list(geometry.verts), list(geometry.faces)
            out[str(product.Tag)] = [
                tuple(tuple(v[3 * k + c] / FOOT for c in range(3)) for k in faces[i : i + 3]) for i in range(0, len(faces), 3)
            ]
    return out


def heights_at(tris, x, y):
    """The heights at which the vertical line through (x, y) meets the
    triangles `tris`: (lowest, highest), or None where it meets none."""
    zs = []
    for (ax, ay, az), (bx, by, bz), (cx, cy, cz) in tris:
        det = (by - cy) * (ax - cx) + (cx - bx) * (ay - cy)
        if abs(det) < 1e-12:
            continue
        l1 = ((by - cy) * (x - cx) + (cx - bx) * (y - cy)) / det
        l2 = ((cy - ay) * (x - cx) + (ax - cx) * (y - cy)) / det
        l3 = 1 - l1 - l2
        if min(l1, l2, l3) >= -1e-9:
            zs.append(l1 * az + l2 * bz + l3 * cz)
    return (min(zs), max(zs)) if zs else None


def ranges(path, kinds=("IfcWall",)):
    """Tag -> (lowest z, highest z in feet, storey name, representation
    types, plan box) of every product of `kinds` with a body."""
    f = ifcopenshell.open(path)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    out = {}
    for wall in (product for kind in kinds for product in f.by_type(kind)):
        if not wall.Tag or wall.Representation is None or str(wall.Tag) in out:
            continue
        try:
            verts = ifcopenshell.geom.create_shape(settings, wall).geometry.verts
        except Exception:
            continue
        if not verts:
            continue
        zs = verts[2::3]
        xs, ys = verts[0::3], verts[1::3]
        container = ifcopenshell.util.element.get_container(wall)
        kinds = sorted(
            {r.RepresentationType or "-" for r in wall.Representation.Representations if r.RepresentationIdentifier == "Body"}
        )
        # IfcOpenShell's geometry is in metres whatever the file's unit.
        out[str(wall.Tag)] = (
            min(zs) / FOOT,
            max(zs) / FOOT,
            container.Name if container is not None and container.is_a("IfcBuildingStorey") else "-",
            "/".join(kinds),
            tuple(v / FOOT for v in (min(zs), max(zs), min(xs), max(xs), min(ys), max(ys))),
        )
    return out


def main():
    args = sys.argv[1:]
    if len(args) != 2:
        print(__doc__)
        return 2
    ours, revit = ranges(args[0]), ranges(args[1])
    both = sorted(set(ours) & set(revit))
    if not both:
        print("no wall in both files")
        return 0
    offset = statistics.median(revit[t][0] - ours[t][0] for t in both)
    counts = collections.Counter()
    storeys = collections.Counter()
    lines = []
    for tag in both:
        low = revit[tag][0] - offset - ours[tag][0]
        high = revit[tag][1] - offset - ours[tag][1]
        base_ok, top_ok = abs(low) <= TOLERANCE, abs(high) <= TOLERANCE
        group = {
            (True, True): "base and top Revit's",
            (True, False): "base Revit's, top not",
            (False, True): "top Revit's, base not",
            (False, False): "neither Revit's",
        }[(base_ok, top_ok)]
        counts[group] += 1
        if not (base_ok and top_ok):
            storeys[("same storey" if ours[tag][2] == revit[tag][2] else "other storey")] += 1
            lines.append(
                f"  tag:{tag} {group}: base {low:+.4f} ft, top {high:+.4f} ft, rvt-rs storey {ours[tag][2]!r},"
                f" Revit storey {revit[tag][2]!r}, Revit body {revit[tag][3]}"
            )
    boxes = cutters(args[0])
    cut = collections.Counter()
    for tag in both:
        base, top = predicted(ours[tag][4], boxes)
        want_base, want_top = revit[tag][0] - offset, revit[tag][1] - offset
        for end, mine, theirs, drawn in (("base", base, want_base, ours[tag][0]), ("top", top, want_top, ours[tag][1])):
            changed = abs(mine - drawn) > TOLERANCE
            right = abs(mine - theirs) <= TOLERANCE
            needed = abs(drawn - theirs) > TOLERANCE
            cut[(end, "cut and Revit's" if changed and right else "cut, not Revit's" if changed else "uncut, Revit cuts" if needed else "uncut and Revit's")] += 1
    print(f"walls in both files: {len(both)} (rvt-rs {len(ours)}, Revit {len(revit)}); offset {offset:.6f} ft")
    # The floors, roofs and footings themselves, which a cut would be read
    # from: are their bases and tops Revit's?
    our_slabs, revit_slabs = ranges(args[0], CUTTERS), ranges(args[1], CUTTERS)
    slab_counts = collections.Counter()
    for tag in sorted(set(our_slabs) & set(revit_slabs)):
        low = revit_slabs[tag][0] - offset - our_slabs[tag][0]
        high = revit_slabs[tag][1] - offset - our_slabs[tag][1]
        ok = (abs(low) <= TOLERANCE, abs(high) <= TOLERANCE)
        slab_counts[ok] += 1
        if ok != (True, True):
            print(f"slab tag:{tag} base {low:+.4f} ft, top {high:+.4f} ft, Revit body {revit_slabs[tag][3]}, rvt-rs body {our_slabs[tag][3]}")
    for (base_ok, top_ok), n in sorted(slab_counts.items()):
        print(f"  floors, roofs and footings with base {'Revit' if base_ok else 'not Revit'}'s and top {'Revit' if top_ok else 'not Revit'}'s: {n}")
    for group, n in counts.most_common():
        print(f"  {group}: {n}")
    for group, n in storeys.most_common():
        print(f"  misses in the {group}: {n}")
    # Every floor, roof or footing whose body lies over a wall's middle in
    # plan, with where its underside and top are there in rvt-rs's body
    # and in Revit's.
    # The files may place the model apart in plan (shared coordinates): a
    # turn and a shift, fitted to the walls' middles by least squares.
    pairs = [
        (
            ((ours[t][4][2] + ours[t][4][3]) / 2, (ours[t][4][4] + ours[t][4][5]) / 2),
            ((revit[t][4][2] + revit[t][4][3]) / 2, (revit[t][4][4] + revit[t][4][5]) / 2),
        )
        for t in both
    ]
    ox = statistics.fmean(a[0] for a, _ in pairs)
    oy = statistics.fmean(a[1] for a, _ in pairs)
    rx = statistics.fmean(b[0] for _, b in pairs)
    ry = statistics.fmean(b[1] for _, b in pairs)
    dot = sum((a[0] - ox) * (b[0] - rx) + (a[1] - oy) * (b[1] - ry) for a, b in pairs)
    cross = sum((a[0] - ox) * (b[1] - ry) - (a[1] - oy) * (b[0] - rx) for a, b in pairs)
    turn = math.atan2(cross, dot)
    cos_t, sin_t = math.cos(turn), math.sin(turn)

    def to_revit(x, y):
        return (rx + cos_t * (x - ox) - sin_t * (y - oy), ry + sin_t * (x - ox) + cos_t * (y - oy))

    spread = statistics.median(math.dist(to_revit(*a), b) for a, b in pairs)
    print(f"plan turn Revit - rvt-rs: {math.degrees(turn):.6f} degrees (median wall-middle residual {spread:.4f} ft)")
    our_surfaces, revit_surfaces = surfaces(args[0]), surfaces(args[1])
    for name, found in (("rvt-rs", our_surfaces), ("Revit", revit_surfaces)):
        print(f"surfaces {name}: {len(found)} bodies, {sum(len(v) for v in found.values())} triangles")
    for probe_tag in ("1861193", "2765485", "1474132"):
        for name, found in (("rvt-rs", our_surfaces), ("Revit", revit_surfaces)):
            tris = found.get(probe_tag, [])
            if tris:
                zs = sorted({round(p[2], 3) for tri in tris for p in tri})
                print(f"surface tag:{probe_tag} {name}: {len(tris)} triangles, heights {zs[:12]}")
    for tag in both:
        box = ours[tag][4]
        mid = ((box[2] + box[3]) / 2, (box[4] + box[5]) / 2)
        found = []
        for other, tris in our_surfaces.items():
            at = heights_at(tris, *mid)
            if at is None or at[1] < box[0] - TOLERANCE or at[0] > box[1] + 5:
                continue
            theirs = heights_at(revit_surfaces.get(other, []), *to_revit(*mid))
            theirs = "-" if theirs is None else f"{theirs[0] - offset:.4f},{theirs[1] - offset:.4f}"
            found.append(f"{other}:{at[0]:.4f},{at[1]:.4f}:{theirs}")
        if found:
            print(f"over tag:{tag} " + " ".join(found))
    print(f"cut by the {len(boxes)} floors, roofs and footings whose plan box overlaps the wall:")
    for (end, group), n in sorted(cut.items()):
        print(f"  {end}: {group}: {n}")
    # Each wall a floor, roof or footing crosses: the crossing's Tag, and
    # whether Revit's end is where it crosses (`cuts`) or not (`passes`).
    for tag in both:
        print(f"wall tag:{tag} rvt-rs " + " ".join(f"{v:.4f}" for v in ours[tag][4]) + f" Revit {revit[tag][0] - offset:.4f} {revit[tag][1] - offset:.4f}")
    for tag in both:
        want = {"base": revit[tag][0] - offset, "top": revit[tag][1] - offset}
        found = crossing(ours[tag][4], boxes)
        if found:
            box = ours[tag][4]
            mid = ((box[2] + box[3]) / 2, (box[4] + box[5]) / 2)
            parts = [
                f"{end}:{other}:{'cuts' if abs(at - want[end]) <= TOLERANCE else 'passes'}:{'over' if covers(other, *mid) else 'beside'}"
                for other, end, at in found
            ]
            print(f"crossed tag:{tag} " + " ".join(parts))
    print("\n".join(lines[:150]))
    return 0


if __name__ == "__main__":
    sys.exit(main())

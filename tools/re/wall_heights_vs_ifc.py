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
import statistics
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element

FOOT = 0.3048
TOLERANCE = 0.001


CUTTERS = ("IfcSlab", "IfcRoof", "IfcFooting")


def extents(f, entity):
    """(lowest z, highest z, x min, x max, y min, y max) of a product's body
    in feet, or None."""
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    try:
        verts = ifcopenshell.geom.create_shape(settings, entity).geometry.verts
    except Exception:
        return None
    if not verts:
        return None
    xs, ys, zs = verts[0::3], verts[1::3], verts[2::3]
    return tuple(v / FOOT for v in (min(zs), max(zs), min(xs), max(xs), min(ys), max(ys)))


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
                print(f"cutter tag:{product.Tag} {kind} {'/'.join(body)} low {box[0]:.4f} high {box[1]:.4f}")
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


def ranges(path):
    """Tag -> (lowest z, highest z in feet, storey name, representation
    types, plan box) of every wall with a body."""
    f = ifcopenshell.open(path)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    out = {}
    for wall in f.by_type("IfcWall"):
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
    for group, n in counts.most_common():
        print(f"  {group}: {n}")
    for group, n in storeys.most_common():
        print(f"  misses in the {group}: {n}")
    print(f"cut by the {len(boxes)} floors, roofs and footings whose plan box overlaps the wall:")
    for (end, group), n in sorted(cut.items()):
        print(f"  {end}: {group}: {n}")
    # Each wall a floor, roof or footing crosses: the crossing's Tag, and
    # whether Revit's end is where it crosses (`cuts`) or not (`passes`).
    for tag in both:
        want = {"base": revit[tag][0] - offset, "top": revit[tag][1] - offset}
        found = crossing(ours[tag][4], boxes)
        if found:
            parts = [f"{end}:{other}:{'cuts' if abs(at - want[end]) <= TOLERANCE else 'passes'}" for other, end, at in found]
            print(f"crossed tag:{tag} " + " ".join(parts))
    print("\n".join(lines[:150]))
    return 0


if __name__ == "__main__":
    sys.exit(main())

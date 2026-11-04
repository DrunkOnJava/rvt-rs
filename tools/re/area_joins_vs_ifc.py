#!/usr/bin/env python3
"""Draw each Area from its area boundary lines and their joins, and score it
against Revit's export (RE-330).

`probe_re317_area_boundaries` prints a Revit 2024 project's area boundary lines
(`line`: box and reference list), its Areas (`area`: GlobalId, number) and the
bytes of each Area's data object (`obj`), whose byte 107 holds the Area's
point. This reads those, applies the rule `partition_area_boundaries` and
`partition_area_records` implement, in Python, and compares the region of each
Area with the profile of its `IfcSpace` in a Revit export, found by GlobalId:

1. a line along a plan axis is read as its box; a line that is not (a slanted
   line, an arc, a line whose box is wider than the line: its record holds only
   its box) is left out, with the lines that name it, and an Area whose
   region's box comes within 0.01 ft of one is refused;
2. an end that lies on no other line is carried on, along its line, to the
   farthest perpendicular line within 1 ft that names it and that it names,
   when that line, once carried on itself, spans the place reached; else the
   end stays free;
3. an end that had no such line is joined to the free end of a parallel line
   within 1 ft that names it and that it names, at the same place along the
   lines and running away on the other side, by a connector, when each is the
   other's only such end;
4. a line whose end is still free then bounds nothing; the Area is the smallest
   loop round its point, less the loops directly inside it.

`--old-edit` moves the 16 lines that Autodesk's `rac_advanced` sample holds
at two positions to the one of its 2015 to 2020 saves and of the public Revit
2021 export of the project (RE-324 section 4, RE-330): the probe is of the
2024 save.

For each Area this prints the symmetric difference of the two regions in ft2
(0.001 or less is the same region), or why it is refused or has no region.

Usage:
    python3 tools/re/area_joins_vs_ifc.py PROBE.txt REVIT.ifc [--old-edit]

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins), NumPy and Shapely.
"""

import math
import struct
import sys

import ifcopenshell
import ifcopenshell.util.placement
import ifcopenshell.util.unit
import numpy as np
from shapely.geometry import LineString, Point, Polygon
from shapely.ops import polygonize, unary_union

from area_boundaries_vs_ifc import elevation_of, profile_points, read_probe, storey_of
from ifc_common import site_frame

TOLERANCE = 1e-5
MAX_CARRY = 1.0
MAX_CONNECTOR = 1.0
SNAP = 1e-4
MARGIN = 0.01
SAME_REGION = 1e-3
POINT_OFFSET = 107

# RE-324 section 4: the lines at two positions, and the y of each position.
EDITED = {
    177354, 177355, 177356, 177365, 177425, 177513,
    178642, 178643, 178644, 178651, 178913,
    179081, 179082, 179083, 179087, 179125,
}  # fmt: skip
NEW_Y = (100.333139, 100.999149, 100.502102, 100.374149)
OLD_Y = (100.357745, 101.023756, 100.526709, 100.398756)


class Line:
    def __init__(self, line_id, box, level, scheme, names):
        self.id = line_id
        self.box = box
        self.level = level
        self.scheme = scheme
        self.names = names
        x0, y0, _, x1, y1, _ = box
        self.a, self.b = (x0, y0), (x1, y1)
        if abs(y1 - y0) <= TOLERANCE and abs(x1 - x0) > TOLERANCE:
            self.axis = "h"
        elif abs(x1 - x0) <= TOLERANCE and abs(y1 - y0) > TOLERANCE:
            self.axis = "v"
        else:
            self.axis = None

    def along(self, point):
        return point[0] if self.axis == "h" else point[1]

    def fixed(self):
        return self.a[1] if self.axis == "h" else self.a[0]

    def ends(self):
        lo, hi = sorted((self.along(self.a), self.along(self.b)))
        return [lo, hi]


def holds(line, point):
    across = point[1] if line.axis == "h" else point[0]
    along = line.along(point)
    lo, hi = line.ends()
    return abs(across - line.fixed()) <= TOLERANCE and lo - TOLERANCE <= along <= hi + TOLERANCE


def point_on(line, along):
    return (along, line.fixed()) if line.axis == "h" else (line.fixed(), along)


def mutual(a, b):
    return b.id in a.names and a.id in b.names


def resolve(readable):
    """The segments of the readable lines once their free ends are carried on
    and joined."""
    by_id = {line.id: line for line in readable}
    ends = {}
    for line in readable:
        for high in (0, 1):
            point = point_on(line, line.ends()[high])
            if not any(o is not line and holds(o, point) for o in readable):
                ends[(line.id, high)] = (line, point)
    reach = {}
    for (line_id, high), (line, point) in ends.items():
        outward = 1 if high else -1
        here = line.ends()[high]
        best = None
        for other in readable:
            if other.axis == line.axis or not mutual(line, other):
                continue
            distance = (other.fixed() - here) * outward
            if distance < -TOLERANCE or distance > MAX_CARRY:
                continue
            if best is None or distance > best[0]:
                best = (distance, other)
        if best:
            reach[(line_id, high)] = (best[1].id, best[1].fixed())
    claimed = set(reach)
    dropped = True
    while dropped:
        dropped = False
        for key in list(reach):
            target_id, _ = reach[key]
            across = by_id[key[0]].fixed()
            low, high = by_id[target_id].ends()
            if (target_id, 0) in reach:
                low = min(low, reach[(target_id, 0)][1])
            if (target_id, 1) in reach:
                high = max(high, reach[(target_id, 1)][1])
            if across < low - TOLERANCE or across > high + TOLERANCE:
                del reach[key]
                dropped = True
    bare = {k: v for k, v in ends.items() if k not in claimed}

    def partners(key):
        line, _ = bare[key]
        here = line.ends()[key[1]]
        return [
            k
            for k, (other, _) in bare.items()
            if other is not line
            and other.axis == line.axis
            and TOLERANCE < abs(other.fixed() - line.fixed()) <= MAX_CONNECTOR
            and k[1] != key[1]
            and mutual(line, other)
            and abs(other.ends()[k[1]] - here) <= TOLERANCE
        ]

    connectors = []
    for key in bare:
        found = partners(key)
        if len(found) == 1 and found[0][0] > key[0] and partners(found[0]) == [key]:
            line, point = bare[key]
            other = bare[found[0]][0]
            along = line.ends()[key[1]]
            low, high = sorted((line.fixed(), other.fixed()))
            if line.axis == "v":
                connectors.append(((low, along), (high, along)))
            else:
                connectors.append(((along, low), (along, high)))
    segments = []
    for line in readable:
        lo, hi = line.ends()
        lo = reach.get((line.id, 0), (None, lo))[1]
        hi = reach.get((line.id, 1), (None, hi))[1]
        segments.append((point_on(line, min(lo, line.ends()[0])), point_on(line, max(hi, line.ends()[1]))))
    return segments + connectors


def snapped(segments):
    """Segments with their ends merged within SNAP and split where an end lies
    on another, so that the union nodes them."""
    reps = []

    def rep(p):
        for r in reps:
            if math.hypot(p[0] - r[0], p[1] - r[1]) <= SNAP:
                return r
        reps.append(tuple(p))
        return tuple(p)

    merged = [[rep(a), rep(b)] for a, b in segments]
    out = []
    for a, b in merged:
        dx, dy = b[0] - a[0], b[1] - a[1]
        square = dx * dx + dy * dy
        inside = []
        for r in reps:
            if r in (a, b) or square == 0:
                continue
            t = ((r[0] - a[0]) * dx + (r[1] - a[1]) * dy) / square
            q = (a[0] + t * dx, a[1] + t * dy)
            if 0 < t < 1 and math.hypot(r[0] - q[0], r[1] - q[1]) <= SNAP:
                inside.append((t, r))
        out.append([a] + [r for _, r in sorted(inside)] + [b])
    return out


def region(segments, point):
    """The smallest loop round `point`, less the loops directly inside it."""
    union = unary_union([LineString(s) for s in snapped(segments)])
    polygons = list(polygonize(union))
    here = Point(point)
    holding = [p for p in polygons if p.contains(here)]
    if not holding:
        return None
    outer = min(holding, key=lambda p: p.area)
    inside = [
        p for p in polygons if p is not outer and outer.buffer(1e-6).contains(p) and not p.contains(here)
    ]
    direct = [p for p in inside if not any(q is not p and q.buffer(1e-6).contains(p) for q in inside)]
    for hole in direct:
        outer = outer.difference(hole)
    return outer


def revit_polygon(space, scale, frame):
    """Revit's profile of the Area, in internal feet: an arbitrary profile, or
    the rectangle Revit writes for a rectangular Area."""
    outer, inners, _ = profile_points(space, scale, frame)
    if outer:
        return Polygon(outer, inners)
    for representation in space.Representation.Representations if space.Representation else []:
        for item in representation.Items:
            if not (item.is_a("IfcExtrudedAreaSolid") and item.SweptArea.is_a("IfcRectangleProfileDef")):
                continue
            profile = item.SweptArea
            matrix = ifcopenshell.util.placement.get_local_placement(space.ObjectPlacement)
            matrix = matrix @ ifcopenshell.util.placement.get_axis2placement(item.Position)
            if profile.Position:
                matrix = matrix @ ifcopenshell.util.placement.get_axis2placement(profile.Position)
            hx, hy = profile.XDim / 2, profile.YDim / 2
            local = np.array([[-hx, -hy, 0, 1], [hx, -hy, 0, 1], [hx, hy, 0, 1], [-hx, hy, 0, 1]])
            world = (matrix @ local.T).T[:, :3] * scale
            return Polygon([(x, y) for x, y, _ in frame(world)])
    return Polygon()


def main(argv):
    args = [a for a in argv[1:] if not a.startswith("--")]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    old_edit = "--old-edit" in argv
    probe = read_probe(args[0])
    model = ifcopenshell.open(args[1])
    frame = site_frame(model)
    scale = ifcopenshell.util.unit.calculate_unit_scale(model)
    spaces = {s.GlobalId: s for s in model.by_type("IfcSpace")}
    levels = {int(f[0]): float(f[2]) for f in probe["level"]}

    schemes = {
        int(r.split(":")[1])
        for f in probe["area"]
        for r in f[7].split(",")
        if r and r.split(":")[2] == "-2003201"
    }
    lines = {}
    for f in probe["line"]:
        if f[1] != "-2000079":
            continue
        box = [float(v) for v in f[7].split(",")]
        refs = [int(v) for v in f[9].split(",") if v]
        level = [r for r in refs if r in levels]
        scheme = [r for r in refs if r in schemes]
        if old_edit and int(f[0]) in EDITED:
            for k in (1, 4):
                for new, old in zip(NEW_Y, OLD_Y):
                    if abs(box[k] - new) < 1e-5:
                        box[k] = old
        if len(level) == 1 and len(scheme) == 1:
            lines[int(f[0])] = Line(int(f[0]), box, level[0], scheme[0], set(refs))

    written = same = 0
    for f in probe["area"]:
        area_id, gid, number = int(f[0]), f[1], f[2]
        space = spaces.get(gid)
        if space is None:
            print(f"Area {number}: not in Revit's export")
            continue
        payload = next(bytes.fromhex(o[-1]) for o in probe["obj"] if o[0] == str(area_id))
        point = struct.unpack_from("<2d", payload, POINT_OFFSET)
        storey = storey_of(space)
        scheme = next(
            int(r.split(":")[1]) for r in f[7].split(",") if r and r.split(":")[2] == "-2003201"
        )
        elevation = elevation_of(storey, scale)
        group = [
            line
            for line in lines.values()
            if line.scheme == scheme and abs(levels[line.level] - elevation) < 1e-3
        ]
        readable = [line for line in group if line.axis]
        unread = [line for line in group if not line.axis]
        unread_ids = {line.id for line in unread}
        blocked = [line.box for line in unread] + [
            line.box for line in readable if line.names & unread_ids
        ]
        shape = region(resolve(readable), point)
        if shape is None:
            print(f"Area {number}: the point is in no loop of the lines")
            continue
        x0, y0, x1, y1 = shape.bounds
        if any(
            min(b[0], b[3]) <= x1 + MARGIN
            and max(b[0], b[3]) >= x0 - MARGIN
            and min(b[1], b[4]) <= y1 + MARGIN
            and max(b[1], b[4]) >= y0 - MARGIN
            for b in blocked
        ):
            print(f"Area {number}: refused, a line that cannot be read lies where its region does")
            continue
        revit = revit_polygon(space, scale, frame)
        difference = shape.symmetric_difference(revit).area
        written += 1
        same += difference <= SAME_REGION
        print(
            f"Area {number}: region {shape.area:.3f} ft2, Revit's {revit.area:.3f},"
            f" symmetric difference {difference:.4f} ft2"
        )
    print(f"{written} Areas drawn, {same} the same region as Revit's (within {SAME_REGION} ft2)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

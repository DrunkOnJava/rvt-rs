#!/usr/bin/env python3
"""Match Revit's IfcSpace profile of each Area with what the file holds (RE-317).

`probe_re317_area_boundaries` prints, for every Area of a Revit 2024 project,
the bytes of its chain record and data objects, where else its ElementId
appears, the chain records it names, and every area boundary line whose
element record decodes (category -2000079, bounding box, references). Revit's
export draws an Area as an `IfcExtrudedAreaSolid` of its boundary. For each
Area, found by Revit's GlobalId, this prints:

- what Revit wrote: the profile class, its curve class, vertex count, holes,
  the extrusion depth, the body's lowest height against the Area's Level, and
  the gap from the Area's storey to the next storey above it;
- the area boundary lines that lie on the outline of Revit's body (both
  ends of a diagonal of the line's bounding box within 0.01 ft of it), how
  much of the outline's length they cover and whether the Area's own record
  names them;
- every `f64` in the probe's byte dumps that equals a coordinate of Revit's
  outline within 0.0002 ft, with the dump and offset it was found at;
- whether the two `f64`s at byte 107 of the Area's data object (the point
  `partition_area_records` reads, `AREA_POINT_OFFSET`) lie inside Revit's
  profile, its holes taken out.

Usage:
    python3 tools/re/area_boundaries_vs_ifc.py PROBE.txt REVIT.ifc [--list]

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins), NumPy and Shapely.
"""

import collections
import struct
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element
import ifcopenshell.util.placement
import ifcopenshell.util.unit
import numpy as np
import shapely
from shapely.geometry import LineString, Point, Polygon

from ifc_common import FEET, site_frame

ON_BOUNDARY = 0.01
COORDINATE_TOLERANCE = 2e-4


def read_probe(path):
    """The probe's lines by kind: tab-separated fields after the first."""
    out = collections.defaultdict(list)
    for line in open(path, encoding="utf-8", errors="replace"):
        parts = line.rstrip("\n").split("\t")
        if len(parts) > 1:
            out[parts[0]].append(parts[1:])
    return out


def storey_of(space):
    parent = ifcopenshell.util.element.get_aggregate(space)
    while parent is not None and not parent.is_a("IfcBuildingStorey"):
        parent = ifcopenshell.util.element.get_aggregate(parent)
    return parent


def elevation_of(storey, scale):
    return float(storey.Elevation) * scale / FEET


def surface_of(settings, frame, space):
    """(top surface, lowest z, highest z) in Revit's internal feet."""
    shape = ifcopenshell.geom.create_shape(settings, space)
    verts = np.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
    faces = np.array(shape.geometry.faces, dtype=int).reshape(-1, 3)
    v = frame(verts)
    a, b, c = v[faces[:, 0]], v[faces[:, 1]], v[faces[:, 2]]
    normal = np.cross(b - a, c - a)
    length = np.linalg.norm(normal, axis=1)
    up = (length > 1e-12) & (normal[:, 2] > 0.9 * length)
    triangles = [
        Polygon([tuple(p[:2]) for p in tri]).buffer(1e-7) for tri in zip(a[up], b[up], c[up])
    ]
    surface = shapely.union_all(triangles) if triangles else Polygon()
    return surface, float(v[:, 2].min()), float(v[:, 2].max())


def profile_of(space, scale):
    """What the body's extrusion is made of, and its depth in feet."""
    out = []
    if not space.Representation:
        return out
    for representation in space.Representation.Representations:
        for item in representation.Items:
            if not item.is_a("IfcExtrudedAreaSolid"):
                continue
            area = item.SweptArea
            curve = getattr(area, "OuterCurve", None)
            holes = len(getattr(area, "InnerCurves", None) or [])
            points = None
            if curve is not None and curve.is_a("IfcPolyline"):
                points = len(curve.Points)
            elif curve is not None and curve.is_a("IfcIndexedPolyCurve"):
                points = len(curve.Points.CoordList)
            out.append(
                (
                    area.is_a(),
                    curve.is_a() if curve is not None else "-",
                    points,
                    holes,
                    float(item.Depth) * scale / FEET,
                )
            )
    return out


def curve_points(curve):
    """The points of a polyline or indexed poly curve, and whether it holds an arc."""
    if curve.is_a("IfcPolyline"):
        return [tuple(point.Coordinates) for point in curve.Points], False
    if curve.is_a("IfcIndexedPolyCurve"):
        points = [tuple(point) for point in curve.Points.CoordList]
        arcs = any(segment.is_a("IfcArcIndex") for segment in curve.Segments or [])
        return points, arcs
    return [], True


def profile_points(space, scale, frame):
    """(outer, inners, arcs) of the first extrusion's profile in Revit's internal
    feet: each curve a list of (x, y), as written, through the extrusion's
    position, the space's placement and the site's."""
    for representation in space.Representation.Representations if space.Representation else []:
        for item in representation.Items:
            if not item.is_a("IfcExtrudedAreaSolid"):
                continue
            area = item.SweptArea
            if not area.is_a("IfcArbitraryClosedProfileDef"):
                continue
            matrix = ifcopenshell.util.placement.get_local_placement(space.ObjectPlacement)
            matrix = matrix @ ifcopenshell.util.placement.get_axis2placement(item.Position)
            out, arcs = [], False
            curves = [area.OuterCurve] + list(getattr(area, "InnerCurves", None) or [])
            for curve in curves:
                points, has_arc = curve_points(curve)
                arcs = arcs or has_arc
                if not points:
                    out.append([])
                    continue
                local = np.array([[x, y, 0.0, 1.0] for x, y in points])
                world = (matrix @ local.T).T[:, :3] * scale
                feet = frame(world)
                out.append([(round(float(x), 6), round(float(y), 6)) for x, y, _ in feet])
            return out[0], out[1:], arcs
    return [], [], False


def rings(surface):
    parts = list(surface.geoms) if hasattr(surface, "geoms") else [surface]
    out = []
    for part in parts:
        if part.is_empty:
            continue
        out.append(list(part.exterior.coords))
        out.extend(list(ring.coords) for ring in part.interiors)
    return out


def hex_blobs(probe, area_id, wanted):
    """(kind, label, bytes) for the dumps that belong to an Area, and for the
    chain records of what its record names."""
    out = []
    for kind in ("rec", "obj", "occ"):
        for fields in probe[kind]:
            if fields[0] != str(area_id):
                continue
            label = " ".join(fields[1:-1])
            out.append((kind, label, bytes.fromhex(fields[-1])))
    for fields in probe["chain"]:
        if int(fields[0]) in wanted:
            label = f"{fields[0]} cat {fields[1]} {fields[2]} {fields[3]}"
            out.append(("chain", label, bytes.fromhex(fields[-1])))
    return out


def coordinate_hits(blob, targets):
    """(offset, value, target) for each f64 of `blob`, at any byte offset, within
    COORDINATE_TOLERANCE of a target."""
    hits = []
    for shift in range(8):
        count = (len(blob) - shift) // 8
        if count <= 0:
            continue
        values = np.frombuffer(blob[shift : shift + 8 * count], dtype="<f8")
        with np.errstate(all="ignore"):
            close = np.abs(values[:, None] - targets[None, :]) <= COORDINATE_TOLERANCE
        for row, column in np.argwhere(close):
            hits.append((shift + 8 * int(row), float(values[row]), float(targets[column])))
    return sorted(set(hits))


def lines_on(boundary, lines):
    """The boundary lines with a diagonal of their box on `boundary`."""
    found = []
    for line_id, category, box in lines:
        x0, y0, _, x1, y1, _ = box
        for a, b in (((x0, y0), (x1, y1)), ((x0, y1), (x1, y0))):
            if (
                boundary.distance(Point(a)) <= ON_BOUNDARY
                and boundary.distance(Point(b)) <= ON_BOUNDARY
                and boundary.distance(Point((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)) <= ON_BOUNDARY
            ):
                found.append((line_id, category, LineString([a, b]).length, box))
                break
    return found


def main(argv):
    args = [a for a in argv[1:] if not a.startswith("--")]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    probe = read_probe(args[0])
    model = ifcopenshell.open(args[1])
    frame = site_frame(model)
    scale = ifcopenshell.util.unit.calculate_unit_scale(model)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    spaces = {space.GlobalId: space for space in model.by_type("IfcSpace")}

    levels = sorted((float(elev), name, int(level_id)) for level_id, name, elev in probe["level"])
    storeys = sorted(
        elevation_of(storey, scale) for storey in model.by_type("IfcBuildingStorey")
        if storey.Elevation is not None
    )
    print(
        "levels (probe): "
        + ", ".join(f"{name} {elev:.4f}" for elev, name, _ in levels)
        + "; storeys (Revit): "
        + ", ".join(f"{e:.4f}" for e in storeys)
    )

    lines = []
    for fields in probe["line"]:
        box = [float(v) for v in fields[7].split(",")]
        lines.append((int(fields[0]), int(fields[1]), box))
    print(
        f"area boundary lines and polylines with a decoded element record: {len(lines)}"
        f" ({sum(1 for _, c, _ in lines if c == -2000079)} lines)"
    )

    counts = collections.Counter()
    for fields in probe["area"]:
        area_id, global_id, number, name, level_id, level_elev, placed, refs = fields[:8]
        area_id = int(area_id)
        space = spaces.get(global_id)
        if space is None:
            counts["not in Revit's export" + ("" if placed == "placed" else " (not placed)")] += 1
            continue
        counts["in Revit's export"] += 1
        wanted = {int(r.split(":")[1]) for r in refs.split(",") if r}
        line_refs = {
            int(r.split(":")[1])
            for r in refs.split(",")
            if r and r.split(":")[2] in ("-2000079", "-2000830")
        }
        profile = profile_of(space, scale)
        storey = storey_of(space)
        storey_elev = (
            elevation_of(storey, scale)
            if storey is not None and storey.Elevation is not None
            else None
        )
        above = [e for e in storeys if storey_elev is not None and e > storey_elev + 1e-6]
        gap = (above[0] - storey_elev) if above else None
        surface, z_low, z_high = surface_of(settings, frame, space)
        quantities = ifcopenshell.util.element.get_psets(space).get("Qto_SpaceBaseQuantities", {})
        height = quantities.get("Height")
        if height is not None:
            height = round(float(height) * scale / FEET, 4)
        elev = None if level_elev == "-" else float(level_elev)
        next_level = [e for e, _, _ in levels if elev is not None and e > elev + 1e-6]
        level_gap = (next_level[0] - elev) if next_level else None
        print(
            f"{area_id} {number} '{name}' Level {level_id} at {level_elev}:"
            f" Revit profile {profile}; body z {z_low:.4f} to {z_high:.4f}"
            f" (lowest minus Level {None if elev is None else round(z_low - elev, 4)});"
            f" depth minus next-Level gap {None if profile == [] or level_gap is None else round(profile[0][4] - level_gap, 4)};"
            f" depth minus next-storey gap {None if profile == [] or gap is None else round(profile[0][4] - gap, 4)};"
            f" Qto Height {height}"
        )
        outer, inners, has_arc = profile_points(space, scale, frame)
        print(f"profile\t{area_id}\t{has_arc}\t{outer}\t{inners}")
        point = None
        for obj in probe["obj"]:
            if obj[0] == str(area_id) and len(obj[-1]) >= 2 * (107 + 16):
                point = struct.unpack_from("<2d", bytes.fromhex(obj[-1]), 107)
        if point is not None and outer:
            inside = Polygon(outer, inners).contains(Point(point))
            print(f"    point {point[0]:.3f} {point[1]:.3f} inside Revit's profile: {inside}")
            counts["point inside Revit's profile" if inside else "point outside Revit's profile"] += 1
        outline = rings(surface)
        if not outline:
            counts["no top surface"] += 1
            continue
        boundary = surface.boundary
        perimeter = float(boundary.length)
        print(
            f"    top surface {surface.area:.3f} ft2, perimeter {perimeter:.3f} ft,"
            f" rings {[len(r) - 1 for r in outline]}"
        )
        on = lines_on(boundary, lines)
        on_ids = {i for i, _, _, _ in on}
        covered = sum(length for _, _, length, _ in on)
        print(
            f"    lines on the outline: {len(on)} of {len(lines)}, length {covered:.3f} of {perimeter:.3f} ft;"
            f" the Area's record names {sorted(line_refs)}, on the outline: {sorted(line_refs & on_ids)}"
        )
        if on:
            print("    " + ", ".join(f"{i}({c})" for i, c, _, _ in on[:24]))
        targets = np.array(sorted({round(c, 6) for ring in outline for p in ring for c in p[:2]}))
        for kind, label, blob in hex_blobs(probe, area_id, wanted):
            for offset, value, target in coordinate_hits(blob, targets)[:6]:
                print(f"    coordinate {value:.6f} (outline {target:.6f}) in {kind} {label} at +{offset:#x}")
                counts[f"coordinate hits in {kind}"] += 1
        if line_refs & on_ids:
            counts["a named line is on the outline"] += 1
        if on:
            counts["lines on the outline"] += 1
    for key in sorted(counts):
        print(f"  {counts[key]:>4}  {key}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

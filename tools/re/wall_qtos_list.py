#!/usr/bin/env python3
"""Which of Revit's wall base quantities a wall's own values give (#1016, W7).

For every wall Revit's export gives `Qto_WallBaseQuantities` and rvt-rs also
writes (by Tag), it checks Revit's quantities against each other (Length
times Height against GrossSideArea, and so on) and against what rvt-rs's own
body of the wall gives: its extrusion's depth, its profile's area and its
extent along the profile's longer side, and the length of its `Axis`
representation. Each line counts the walls where the two are equal (a
relative 1e-5, or 1e-6 absolute in SI units), and lists up to 8 where they
are not.

Usage:

    python3 tools/re/wall_qtos_list.py <rvt-rs.ifc> <revit-export.ifc>
"""

import collections
import math
import os
import sys

import ifcopenshell
import ifcopenshell.util.unit

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from qtos_vs_ifc import quantities  # noqa: E402

SET = "Qto_WallBaseQuantities"


def points_of(curve, scale):
    if curve.is_a("IfcPolyline"):
        return [(p.Coordinates[0] * scale, p.Coordinates[1] * scale) for p in curve.Points]
    if curve.is_a("IfcIndexedPolyCurve"):
        return [(c[0] * scale, c[1] * scale) for c in curve.Points.CoordList]
    return None


def shoelace(points):
    if not points:
        return None
    area = 0.0
    for (x0, y0), (x1, y1) in zip(points, points[1:] + points[:1]):
        area += x0 * y1 - x1 * y0
    return abs(area) / 2


def profile(p, scale):
    """(area, extent along the longer side) of a profile, in SI units."""
    if p.is_a("IfcRectangleProfileDef"):
        x, y = p.XDim * scale, p.YDim * scale
        return x * y, max(x, y)
    if p.is_a("IfcArbitraryClosedProfileDef"):
        outer = points_of(p.OuterCurve, scale)
        if outer is None:
            return None, None
        area = shoelace(outer)
        for inner in getattr(p, "InnerCurves", None) or ():
            pts = points_of(inner, scale)
            if pts is None:
                return None, None
            area -= shoelace(pts)
        xs = [x for x, _ in outer]
        ys = [y for _, y in outer]
        return area, max(max(xs) - min(xs), max(ys) - min(ys))
    return None, None


def solids(item):
    if item.is_a("IfcExtrudedAreaSolid"):
        yield item
    elif item.is_a("IfcBooleanResult"):
        yield from solids(item.FirstOperand)


def ours(path):
    f = ifcopenshell.open(path)
    scale = ifcopenshell.util.unit.calculate_unit_scale(f)
    out = {}
    for wall in f.by_type("IfcWall"):
        if not wall.Tag or wall.Representation is None:
            continue
        depth = area = extent = axis = None
        items = 0
        for rep in wall.Representation.Representations:
            if rep.RepresentationIdentifier == "Body":
                for item in rep.Items:
                    for solid in solids(item):
                        items += 1
                        d = solid.Depth * scale
                        a, e = profile(solid.SweptArea, scale)
                        depth = d if depth is None else max(depth, d)
                        area = a if area is None or a is None else area + a
                        extent = e if extent is None or e is None else max(extent, e)
            elif rep.RepresentationIdentifier == "Axis":
                for item in rep.Items:
                    pts = points_of(item, scale) if item.is_a() in ("IfcPolyline", "IfcIndexedPolyCurve") else None
                    if pts:
                        axis = sum(math.dist(a, b) for a, b in zip(pts, pts[1:]))
        out[f"tag:{wall.Tag}"] = {
            "depth": depth,
            "profile area": area,
            "profile extent": extent,
            "axis length": axis,
            "solids": items,
        }
    return out


def same(a, b):
    return a is not None and b is not None and abs(a - b) <= max(1e-6, 1e-5 * max(abs(a), abs(b)))


def main(argv):
    args = [a for a in argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    mine = ours(args[0])
    theirs = collections.defaultdict(dict)
    for (key, qset, name), (_, value) in quantities(args[1])[0].items():
        if qset == SET and value is not None:
            theirs[key][name] = value
    walls = {k: v for k, v in theirs.items() if k in mine}
    print(f"walls with {SET} in Revit's export that rvt-rs writes: {len(walls)} (of {len(theirs)})")

    def rvt(key, name):
        return mine[key][name]

    def q(key, name):
        return walls[key].get(name)

    checks = [
        ("Height = rvt-rs body depth", "Height", lambda k: rvt(k, "depth")),
        ("Length = rvt-rs axis length", "Length", lambda k: rvt(k, "axis length")),
        ("Length = rvt-rs profile extent", "Length", lambda k: rvt(k, "profile extent")),
        ("GrossFootprintArea = rvt-rs profile area", "GrossFootprintArea", lambda k: rvt(k, "profile area")),
        ("NetFootprintArea = rvt-rs profile area", "NetFootprintArea", lambda k: rvt(k, "profile area")),
        ("GrossVolume = rvt-rs profile area x depth", "GrossVolume",
         lambda k: None if rvt(k, "profile area") is None or rvt(k, "depth") is None else rvt(k, "profile area") * rvt(k, "depth")),
        ("GrossSideArea = Length x Height", "GrossSideArea",
         lambda k: None if q(k, "Length") is None or q(k, "Height") is None else q(k, "Length") * q(k, "Height")),
        ("GrossFootprintArea = Length x Width", "GrossFootprintArea",
         lambda k: None if q(k, "Length") is None or q(k, "Width") is None else q(k, "Length") * q(k, "Width")),
        ("GrossVolume = GrossFootprintArea x Height", "GrossVolume",
         lambda k: None if q(k, "GrossFootprintArea") is None or q(k, "Height") is None else q(k, "GrossFootprintArea") * q(k, "Height")),
        ("GrossVolume = GrossSideArea x Width", "GrossVolume",
         lambda k: None if q(k, "GrossSideArea") is None or q(k, "Width") is None else q(k, "GrossSideArea") * q(k, "Width")),
        ("NetVolume = NetSideArea x Width", "NetVolume",
         lambda k: None if q(k, "NetSideArea") is None or q(k, "Width") is None else q(k, "NetSideArea") * q(k, "Width")),
        ("NetVolume = GrossVolume", "NetVolume", lambda k: q(k, "GrossVolume")),
        ("NetSideArea = GrossSideArea", "NetSideArea", lambda k: q(k, "GrossSideArea")),
    ]
    for label, name, other in checks:
        pairs = [(k, q(k, name), other(k)) for k in sorted(walls)]
        pairs = [(k, a, b) for k, a, b in pairs if a is not None and b is not None]
        equal = [p for p in pairs if same(p[1], p[2])]
        print(f"{label}: {len(equal)} of {len(pairs)}")
        for k, a, b in [p for p in pairs if not same(p[1], p[2])][:8]:
            print(f"    {k}: Revit {a:.6g}, other {b:.6g} (solids {mine[k]['solids']})")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

#!/usr/bin/env python3
"""Print Revit's body of each element a probe prints as `elem <id>` (W6).

For each `elem <id>` line of the probe output, the element with that Tag in
Revit's export: its class and ObjectType, its placement's origin and axes,
and every item of its Body representation (an extrusion's profile with its
parameters, depth and direction; any other item's type), so the native
records the probe prints can be read against the body Revit draws.

Usage:

    python3 tools/re/mullion_bodies_list.py <probe output> <revit.ifc> [--list]
"""

import sys

import ifcopenshell
import ifcopenshell.util.placement

FT = 0.3048


def describe(item, scale):
    if item.is_a("IfcMappedItem"):
        return "mapped[" + "; ".join(describe(i, scale) for i in item.MappingSource.MappedRepresentation.Items) + "]"
    if item.is_a("IfcExtrudedAreaSolid"):
        profile = item.SweptArea
        attrs = []
        for name in ("XDim", "YDim", "Radius", "Depth", "Width", "WallThickness", "FlangeThickness", "WebThickness"):
            value = getattr(profile, name, None)
            if isinstance(value, float):
                attrs.append(f"{name}={value * scale / FT:.6f}")
        if profile.is_a("IfcArbitraryClosedProfileDef"):
            curves = [("outer", profile.OuterCurve)] + [
                ("inner", c) for c in (getattr(profile, "InnerCurves", None) or [])
            ]
            for label, curve in curves:
                points = getattr(curve, "Points", None)
                coords = list(points.CoordList) if points is not None and hasattr(points, "CoordList") else []
                xs = [p[0] * scale / FT for p in coords]
                ys = [p[1] * scale / FT for p in coords]
                segs = [s.is_a() for s in (getattr(curve, "Segments", None) or [])]
                extent = (
                    f" x {min(xs):.6f}..{max(xs):.6f} y {min(ys):.6f}..{max(ys):.6f}" if coords else ""
                )
                pts = " ".join(f"({p[0] * scale / FT:.6f},{p[1] * scale / FT:.6f})" for p in coords)
                attrs.append(f"{label}={curve.is_a()} points={len(coords)}{extent} segments={segs} coords={pts}")
        direction = tuple(round(v, 6) for v in item.ExtrudedDirection.DirectionRatios)
        return f"extrusion {profile.is_a()} {' '.join(attrs)} depth={item.Depth * scale / FT:.6f} dir={direction}"
    return item.is_a()


def main():
    ids = [line.split()[1] for line in open(sys.argv[1], encoding="utf-8", errors="replace") if line.startswith("elem ")]
    model = ifcopenshell.open(sys.argv[2])
    scale = ifcopenshell.util.unit.calculate_unit_scale(model) if hasattr(ifcopenshell.util, "unit") else 1.0
    by_tag = {}
    for element in model.by_type("IfcElement"):
        if element.Tag:
            by_tag.setdefault(str(element.Tag), []).append(element)
    for tag in ids:
        for element in by_tag.get(tag, []):
            matrix = ifcopenshell.util.placement.get_local_placement(element.ObjectPlacement)
            origin = [round(matrix[i][3] * scale / FT, 4) for i in range(3)]
            x_axis = [round(matrix[i][0], 4) for i in range(3)]
            z_axis = [round(matrix[i][2], 4) for i in range(3)]
            print(f"elem {tag}: {element.is_a()} ObjectType {element.ObjectType!r} origin {origin} x {x_axis} z {z_axis}")
            for rep in (element.Representation.Representations if element.Representation else []):
                if rep.RepresentationIdentifier != "Body":
                    continue
                for item in rep.Items:
                    print(f"  {describe(item, scale)}")
        if tag not in by_tag:
            print(f"elem {tag}: not in Revit's export")
    return 0


if __name__ == "__main__":
    import ifcopenshell.util.unit  # noqa: F401

    sys.exit(main())

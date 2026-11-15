#!/usr/bin/env python3
"""For a few walls, the length of the `Axis` curve Revit's export writes and
the extent of its `Body` along that axis, in the wall's own frame (#1407).

Research tool. revit-ifc (CLEANROOM.md source 5, `WallExporter`) makes a wall
it can extrude from its trimmed location curve thickened by its width
(`GetBoundaryLoopsFromWall`), or from its base curve where the footprint is
short (`GetBoundaryLoopsFromBaseCurve`), and writes that trimmed curve as the
`Axis`. Revit's own quantities and the saved mesh are the cleaned solid. This
prints, for each wall of FOCUS, the Axis's end points and length, the Body's
items and their extent along the Axis, and the native `GLine` lines the probe
`probe_1067_native_dump` prints for the wall (its location line).

Usage: wall_axis_body_vs_ifc.py <probe output> <revit-export.ifc> [--list]
"""

import json
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.placement
import ifcopenshell.util.unit
import numpy as np

FOCUS = ("2177212",)
FEET = 0.3048


def main(argv):
    positional = [a for a in argv[1:] if not a.startswith("--")]
    if len(positional) != 2:
        sys.exit(__doc__)
    probe, theirs = positional
    f = ifcopenshell.open(theirs)
    scale = ifcopenshell.util.unit.calculate_unit_scale(f)
    settings = ifcopenshell.geom.settings()
    for wall in f.by_type("IfcWall"):
        if wall.Tag not in FOCUS:
            continue
        m = ifcopenshell.util.placement.get_local_placement(wall.ObjectPlacement)
        print(f"wall {wall.Tag} placement x axis {np.round(m[:3, 0], 6).tolist()} origin {np.round(m[:3, 3] * scale / FEET, 4).tolist()} ft")
        for rep in wall.Representation.Representations:
            for item in rep.Items:
                print(f"  {rep.RepresentationIdentifier} {rep.RepresentationType} {item.is_a()}")
                if item.is_a("IfcPolyline") or item.is_a("IfcIndexedPolyCurve") or item.is_a("IfcTrimmedCurve"):
                    if item.is_a("IfcPolyline"):
                        pts = [np.array(p.Coordinates, dtype=float) * scale / FEET for p in item.Points]
                    elif item.is_a("IfcIndexedPolyCurve"):
                        pts = [np.array(p, dtype=float) * scale / FEET for p in item.Points.CoordList]
                    else:
                        pts = []
                    if pts:
                        length = sum(float(np.linalg.norm(b - a)) for a, b in zip(pts, pts[1:]))
                        print(f"    points {[np.round(p, 4).tolist() for p in pts]} length {length:.4f} ft")
                if item.is_a("IfcExtrudedAreaSolid") or item.is_a("IfcBooleanClippingResult") or item.is_a("IfcBooleanResult"):
                    base = item
                    while base.is_a("IfcBooleanResult"):
                        base = base.FirstOperand
                    if base.is_a("IfcExtrudedAreaSolid"):
                        prof = base.SweptArea
                        print(f"    base extrusion depth {base.Depth * scale / FEET:.4f} ft profile {prof.is_a()}")
                        if prof.is_a("IfcRectangleProfileDef"):
                            print(f"    rectangle {prof.XDim * scale / FEET:.4f} by {prof.YDim * scale / FEET:.4f} ft")
                        elif prof.is_a("IfcArbitraryClosedProfileDef"):
                            curve = prof.OuterCurve
                            if curve.is_a("IfcPolyline"):
                                pts = np.array([p.Coordinates[:2] for p in curve.Points], dtype=float) * scale / FEET
                            elif curve.is_a("IfcIndexedPolyCurve"):
                                pts = np.array([p[:2] for p in curve.Points.CoordList], dtype=float) * scale / FEET
                            else:
                                pts = None
                            if pts is not None:
                                print(f"    profile x {pts[:, 0].min():.4f}..{pts[:, 0].max():.4f} y {pts[:, 1].min():.4f}..{pts[:, 1].max():.4f} ft (local)")
        try:
            shape = ifcopenshell.geom.create_shape(settings, wall)
            v = np.array(shape.geometry.verts, dtype=float).reshape(-1, 3) / FEET
            # Extent along the placement's x axis in the body's own frame.
            print(f"  body local x {v[:, 0].min():.4f}..{v[:, 0].max():.4f} length {v[:, 0].max() - v[:, 0].min():.4f} ft; z {v[:, 2].min():.4f}..{v[:, 2].max():.4f}")
        except Exception as error:  # noqa: BLE001
            print(f"  body not meshed: {error}")
    with open(probe) as handle:
        for line in handle:
            parts = line.split(" ", 3)
            if len(parts) >= 4 and parts[0] == "wall" and parts[1] in FOCUS and ("GLine" in line or " root " in line):
                text = line.strip()
                print("probe " + (text if len(text) < 1500 else text[:1500]))


if __name__ == "__main__":
    main(sys.argv)

#!/usr/bin/env python3
"""Research trace for #1208 (not to land): for each slab or covering whose
`Width` or `Length` base quantity Revit's export writes, the axes Revit's body
extrusion is drawn in (the swept area's X and Y in Revit's internal frame), the
profile's extents along them, the quantities, and rvt-rs's placement X and its
body's extents along its own X and Y, so the plane and axes Revit picks can be
compared with what rvt-rs reads.

    python3 tools/re/trace_1208_slab_axes.py <rvt-rs.ifc> <revit-export.ifc>
"""

import os
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element
import ifcopenshell.util.placement
import ifcopenshell.util.unit
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ifc_common import FEET, site_frame  # noqa: E402

SETS = ("Qto_SlabBaseQuantities", "Qto_CoveringBaseQuantities")


def directions(frame, unit, matrix, vectors):
    o = matrix[:3, 3] * unit
    pts = frame(np.array([o] + [o + matrix[:3, :3] @ v for v in vectors]))
    return pts[0], [(p - pts[0]) / max(np.linalg.norm(p - pts[0]), 1e-12) for p in pts[1:]]


def body_items(element):
    rep = element.Representation
    for r in rep.Representations if rep else []:
        if r.RepresentationIdentifier == "Body":
            for item in r.Items:
                while item.is_a("IfcMappedItem"):
                    item = item.MappingSource.MappedRepresentation.Items[0]
                yield item


def facts(path):
    f = ifcopenshell.open(path)
    frame = site_frame(f)
    unit = ifcopenshell.util.unit.calculate_unit_scale(f)
    out = {}
    for e in list(f.by_type("IfcSlab")) + list(f.by_type("IfcCovering")):
        if not e.Tag:
            continue
        psets = ifcopenshell.util.element.get_psets(e)
        q = next((psets[s] for s in SETS if s in psets), {})
        mat = np.array(ifcopenshell.util.placement.get_local_placement(e.ObjectPlacement), dtype=float)
        origin, (px, py) = directions(frame, unit, mat, [np.array([1.0, 0, 0]), np.array([0, 1.0, 0])])
        prof = None
        for item in body_items(e):
            if item.is_a("IfcExtrudedAreaSolid"):
                pos = np.array(ifcopenshell.util.placement.get_axis2placement(item.Position), dtype=float)
                area = item.SweptArea
                apos = getattr(area, "Position", None)
                amat = np.eye(4)
                if apos is not None:
                    amat = np.array(ifcopenshell.util.placement.get_axis2placement(apos), dtype=float)
                    if amat.shape == (3, 3):
                        m4 = np.eye(4)
                        m4[:2, :2] = amat[:2, :2]
                        m4[:2, 3] = amat[:2, 2]
                        amat = m4
                full = mat @ pos @ amat
                _, (ax, ay) = directions(frame, unit, full, [np.array([1.0, 0, 0]), np.array([0, 1.0, 0])])
                pts = None
                if area.is_a("IfcArbitraryClosedProfileDef"):
                    curve = area.OuterCurve
                    if curve.is_a("IfcPolyline"):
                        pts = np.array([p.Coordinates[:2] for p in curve.Points], dtype=float)
                    elif curve.is_a("IfcIndexedPolyCurve"):
                        pts = np.array([c[:2] for c in curve.Points.CoordList], dtype=float)
                elif area.is_a("IfcRectangleProfileDef"):
                    pts = np.array([[-area.XDim / 2, -area.YDim / 2], [area.XDim / 2, area.YDim / 2]])
                ext = None if pts is None else (pts.max(axis=0) - pts.min(axis=0)) * unit / FEET
                prof = (area.is_a(), ax, ay, ext, item.Depth * unit / FEET)
                break
        out[e.Tag] = {
            "class": e.is_a(),
            "type": e.ObjectType,
            "width": None if q.get("Width") is None else q["Width"] * unit / FEET,
            "length": None if q.get("Length") is None else q["Length"] * unit / FEET,
            "x": px,
            "profile": prof,
        }
    return out


def main(argv):
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    ours, revit = facts(argv[1]), facts(argv[2])
    vec = lambda v: "-" if v is None else f"{v[0]:+.3f} {v[1]:+.3f} {v[2]:+.3f}"
    num = lambda v: "-" if v is None else f"{v:.4f}"
    shown = 0
    for tag, r in sorted(revit.items()):
        if r["width"] is None and r["length"] is None:
            continue
        shown += 1
        o = ours.get(tag)
        p = r["profile"]
        print(f"trace 1208 {tag} {r['class']} {r['type']!r}: Revit Width {num(r['width'])} Length {num(r['length'])}"
              f"; rvt-rs Width {num(o and o['width'])} Length {num(o and o['length'])}")
        if p:
            print(f"  Revit profile {p[0]} x {vec(p[1])} y {vec(p[2])} extent {'-' if p[3] is None else f'{p[3][0]:.4f} {p[3][1]:.4f}'} depth {p[4]:.4f}")
        print(f"  Revit placement x {vec(r['x'])}")
        if o:
            q = o["profile"]
            print(f"  rvt-rs placement x {vec(o['x'])}"
                  + (f"; profile {q[0]} x {vec(q[1])} extent {'-' if q[3] is None else f'{q[3][0]:.4f} {q[3][1]:.4f}'}" if q else ""))
    print(f"trace 1208: slabs and coverings with Revit's Width or Length: {shown}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

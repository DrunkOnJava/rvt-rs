#!/usr/bin/env python3
"""Compare the coordinate frame of an rvt-rs IFC with Revit's export of the same model.

Revit's IFC exporter places a model by the export setup's coordinate base
(internal origin, project base point, survey point or shared coordinates):
the same model exports at different offsets and rotations, and the setup is
recorded only in the file's header (`CoordinateReference` in FILE_DESCRIPTION
on Revit 2024 and later). rvt-rs writes Revit's internal frame: IfcSite at
the identity placement, no IfcMapConversion. The other scorers compare the
two by undoing Revit's IfcSite placement (`opening_boxes_vs_ifc.site_frame`).
This says, per model, whether that holds.

It prints each file's coordinate reference, IfcMapConversion and site
placement (translation in metres, rotation about Z in degrees), then fits the
rigid transform about Z that carries rvt-rs's body centres onto Revit's,
from the bounding-box centres of up to 200 elements both files tag with the
same ElementId (walls, columns, slabs, roofs, beams), and the residual left
after it and after Revit's site placement alone. Lengths in metres.

An element is every product of its class with its Tag (a floor Revit splits
into slabs taken whole). Revit's boxes are taken in rvt-rs's orientation:
its vertices with its site placement undone, the centre then placed again, so
a site rotation turns each centre with its body.

Usage:

    python3 tools/re/frames_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import math
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.placement
import ifcopenshell.util.unit
import numpy as np

CLASSES = ("IfcWall", "IfcColumn", "IfcSlab", "IfcRoof", "IfcBeam")
SAMPLE = 200


def header(f, label):
    reference = [d.strip() for d in (f.header.file_description.description or ()) if "Coordinate" in d]
    print(f"{label} coordinate reference: {'; '.join(reference) or 'none in the header'}")
    conversions = f.by_type("IfcMapConversion") if f.schema != "IFC2X3" else []
    for c in conversions:
        print(
            f"{label} IfcMapConversion: eastings {c.Eastings:.3f} northings {c.Northings:.3f}"
            f" height {c.OrthogonalHeight:.3f} x-axis ({axis(c.XAxisAbscissa, 1):.6f}, {axis(c.XAxisOrdinate, 0):.6f})"
        )
    if not conversions:
        print(f"{label} IfcMapConversion: none")
    site = f.by_type("IfcSite")
    if not site or site[0].ObjectPlacement is None:
        print(f"{label} site placement: none")
        return np.eye(4)
    m = np.array(ifcopenshell.util.placement.get_local_placement(site[0].ObjectPlacement), dtype=float)
    scale = ifcopenshell.util.unit.calculate_unit_scale(f)
    angle = math.degrees(math.atan2(m[1][0], m[0][0]))
    t = m[:3, 3] * scale
    print(f"{label} site placement: translation ({t[0]:.3f}, {t[1]:.3f}, {t[2]:.3f}) rotation {angle:.4f} deg")
    m[:3, 3] = t
    return m


def axis(value, unset):
    return unset if value is None else value


def centres(f, tags, frame):
    """Tag -> the centre of the bounding box of every product of CLASSES with
    that Tag, taken with `frame` undone and then placed by it again."""
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    undo = np.linalg.inv(frame)
    verts = {}
    for element in (e for c in CLASSES for e in f.by_type(c)):
        tag = element.Tag
        if tag not in tags:
            continue
        try:
            shape = ifcopenshell.geom.create_shape(settings, element)
        except Exception:
            continue
        v = np.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
        if len(v):
            verts.setdefault(tag, []).append(v)
    out = {}
    for tag, pieces in verts.items():
        v = np.vstack(pieces)
        local = (undo[:3, :3] @ v.T).T + undo[:3, 3]
        centre = (local.min(axis=0) + local.max(axis=0)) / 2
        out[tag] = frame[:3, :3] @ centre + frame[:3, 3]
    return out


def tagged(f):
    return {e.Tag for c in CLASSES for e in f.by_type(c) if e.Tag}


def fit(a, b):
    """Rotation about Z and translation carrying points a onto b (Kabsch in
    plan, mean offset in Z)."""
    ca, cb = a.mean(axis=0), b.mean(axis=0)
    p, q = a[:, :2] - ca[:2], b[:, :2] - cb[:2]
    h = p.T @ q
    angle = math.atan2(h[0, 1] - h[1, 0], h[0, 0] + h[1, 1])
    c, s = math.cos(angle), math.sin(angle)
    r = np.array([[c, -s], [s, c]])
    t = np.append(cb[:2] - r @ ca[:2], cb[2] - ca[2])
    moved = np.column_stack([(r @ a[:, :2].T).T + t[:2], a[:, 2] + t[2]])
    return math.degrees(angle), t, np.linalg.norm(moved - b, axis=1)


def main(argv):
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    ours, theirs = ifcopenshell.open(argv[1]), ifcopenshell.open(argv[2])
    header(ours, "rvt-rs")
    site = header(theirs, "Revit")
    common = sorted(tagged(ours) & tagged(theirs), key=lambda t: (len(t), t))
    step = max(1, -(-len(common) // SAMPLE))
    sample = set(common[::step])
    a_all, b_all = centres(ours, sample, np.eye(4)), centres(theirs, sample, site)
    both = sorted(set(a_all) & set(b_all), key=lambda t: (len(t), t))
    print(f"elements tagged in both: {len(common)}; centres compared: {len(both)}")
    if len(both) < 3:
        print("too few to fit a frame")
        return 0
    a = np.array([a_all[t] for t in both])
    b = np.array([b_all[t] for t in both])
    angle, t, residual = fit(a, b)
    print(
        f"fitted rvt-rs -> Revit: translation ({t[0]:.3f}, {t[1]:.3f}, {t[2]:.3f}) rotation {angle:.4f} deg;"
        f" residual median {np.median(residual):.4f} max {residual.max():.4f}"
    )
    by_site = (site[:3, :3] @ a.T).T + site[:3, 3]
    off = np.linalg.norm(by_site - b, axis=1)
    print(f"Revit's site placement applied to rvt-rs: residual median {np.median(off):.4f} max {off.max():.4f}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

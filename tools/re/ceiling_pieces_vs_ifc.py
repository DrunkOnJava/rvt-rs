#!/usr/bin/env python3
"""Which piece of a ceiling in pieces Revit's GrossArea is (#1209, RE-297 section 2).

A ceiling whose sketch is several loops is written by rvt-rs as several
IfcCovering pieces sharing one Tag (#331). Revit's export writes one
IfcCovering with one `Qto_CoveringBaseQuantities.GrossArea`, the area of one
piece, and RE-297 found nothing in the record that says which. For every Tag
rvt-rs writes two or more coverings for, this prints each piece in the order
rvt-rs writes them (its sketch's order) with its plan area (its volume over its
extrusion's depth, holes taken off), marks the piece whose area is Revit's
GrossArea, and says which pieces hold the plan point of Revit's own placement:
a hypothesis for the rule (the piece around the element's origin), scored
here, not assumed. A tally ends it: matches by position (first, last, largest,
smallest) and by the origin.

Usage:

    python3 tools/re/ceiling_pieces_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import collections
import os
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element
import ifcopenshell.util.placement
import ifcopenshell.util.unit
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ifc_common import site_frame  # noqa: E402

# Two areas are one where they differ by less than this, relative (Revit's GrossArea is written to a few digits).
AREA_RELATIVE = 1e-3


def depth_of(product):
    """The depth of the first extrusion of the product's body, metres, or None."""
    unit = ifcopenshell.util.unit.calculate_unit_scale(product.wrapped_data.file)
    for rep in (product.Representation.Representations if product.Representation else []):
        for item in rep.Items:
            if item.is_a("IfcExtrudedAreaSolid"):
                return item.Depth * unit
            if item.is_a("IfcMappedItem"):
                for inner in item.MappingSource.MappedRepresentation.Items:
                    if inner.is_a("IfcExtrudedAreaSolid"):
                        return inner.Depth * unit
    return None


def pieces(path):
    """Tag -> [(area m2, plan triangles in Revit feet)] for every IfcCovering, in file order."""
    f = ifcopenshell.open(path)
    frame = site_frame(f)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    out = collections.defaultdict(list)
    for c in f.by_type("IfcCovering"):
        if not c.Tag or not c.Representation:
            continue
        depth = depth_of(c)
        try:
            shape = ifcopenshell.geom.create_shape(settings, c)
        except Exception:
            out[c.Tag].append((None, None))
            continue
        verts = np.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
        faces = np.array(shape.geometry.faces, dtype=int).reshape(-1, 3)
        tri = verts[faces]
        cross = np.cross(tri[:, 1] - tri[:, 0], tri[:, 2] - tri[:, 0])
        volume = abs(np.einsum("ij,ij->i", tri[:, 0], cross).sum()) / 6.0
        area = volume / depth if depth else None
        up = tri[cross[:, 2] > 1e-12]
        plan = frame(up.reshape(-1, 3)).reshape(-1, 3, 3)[:, :, :2] if len(up) else None
        out[c.Tag].append((area, plan))
    return f, out


def inside(point, triangles):
    """Whether `point` (x, y feet) lies in any of the plan triangles."""
    if triangles is None:
        return False
    p = np.asarray(point)
    a, b, c = triangles[:, 0], triangles[:, 1], triangles[:, 2]

    def side(u, v):
        return (v[:, 0] - u[:, 0]) * (p[1] - u[:, 1]) - (v[:, 1] - u[:, 1]) * (p[0] - u[:, 0])

    s1, s2, s3 = side(a, b), side(b, c), side(c, a)
    eps = 1e-9
    return bool((((s1 >= -eps) & (s2 >= -eps) & (s3 >= -eps)) | ((s1 <= eps) & (s2 <= eps) & (s3 <= eps))).any())


def gross_area(product):
    for rel in getattr(product, "IsDefinedBy", []) or []:
        if not rel.is_a("IfcRelDefinesByProperties"):
            continue
        qset = rel.RelatingPropertyDefinition
        if qset.is_a("IfcElementQuantity") and qset.Name == "Qto_CoveringBaseQuantities":
            for q in qset.Quantities:
                if q.is_a("IfcQuantityArea") and q.Name == "GrossArea":
                    return q.AreaValue
    return None


def main(argv):
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    _, ours = pieces(argv[1])
    revit = ifcopenshell.open(argv[2])
    unit = ifcopenshell.util.unit.calculate_unit_scale(revit)
    frame = site_frame(revit)
    tally = collections.Counter()
    compared = 0
    for c in revit.by_type("IfcCovering"):
        mine = ours.get(c.Tag)
        if not mine or len(mine) < 2:
            continue
        gross = gross_area(c)
        if gross is None:
            continue
        gross *= unit * unit
        origin = ifcopenshell.util.placement.get_local_placement(c.ObjectPlacement)[:3, 3] * unit
        at = frame(np.array([origin]))[0][:2]
        areas = [area for area, _ in mine]
        match = [i for i, area in enumerate(areas) if area and abs(area - gross) <= AREA_RELATIVE * gross]
        holds = [i for i, (_, plan) in enumerate(mine) if inside(at, plan)]
        compared += 1
        known = [a for a in areas if a]
        positions = []
        if match:
            m = match[0]
            positions.append("first" if m == 0 else "last" if m == len(areas) - 1 else "middle")
            if known and areas[m] == max(known):
                positions.append("largest")
            if known and areas[m] == min(known):
                positions.append("smallest")
            positions.append("holds the origin" if m in holds else "not the origin's")
        for p in positions or ["no piece matches"]:
            tally[p] += 1
        shown = ", ".join(f"{a:.4f}" if a else "-" for a in areas)
        print(
            f"ceiling tag:{c.Tag} pieces {len(areas)} areas [{shown}] Revit GrossArea {gross:.4f} "
            f"matches {match or '-'} origin in {holds or '-'}"
        )
    print(
        f"summary ceilings in pieces compared {compared}: "
        + ", ".join(f"{k} {v}" for k, v in sorted(tally.items()))
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

#!/usr/bin/env python3
"""List the openings an IFC export cuts in roofs, slabs and coverings, with
their bodies (#824).

Takes any probe output and Revit's IFC export, so Measure can run it as a
probe scorer. For Revit's export, and then for the rvt-rs export beside the
probe output (`model.ifc`, kept by Measure's `keep_ifc`) where there is one,
it prints every `IfcOpeningElement` that voids an element other than a
wall: the opening's Tag, Name and GlobalId, its host's class, Tag and Name,
its world box in the export's units, and each body item: its class and, for
an extrusion, its profile, depth and direction.

Usage:

    python3 tools/re/host_openings_list.py <probe.txt> <revit.ifc> [--list]
"""

import os
import sys

import ifcopenshell
import ifcopenshell.geom
import numpy as np


def describe(item):
    if item.is_a("IfcExtrudedAreaSolid"):
        area = item.SweptArea
        if area.is_a("IfcRectangleProfileDef"):
            profile = f"rectangle {area.XDim:.4f} x {area.YDim:.4f}"
        else:
            profile = area.is_a()
        d = item.ExtrudedDirection.DirectionRatios
        return f"extrusion {profile}, depth {item.Depth:.4f}, direction ({d[0]:.3f}, {d[1]:.3f}, {d[2]:.3f})"
    if item.is_a("IfcMappedItem"):
        return "mapped: " + "; ".join(describe(i) for i in item.MappingSource.MappedRepresentation.Items)
    return item.is_a()


def listing(label, path):
    model = ifcopenshell.open(path)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    count = 0
    print(f"== {label}")
    for rel in model.by_type("IfcRelVoidsElement"):
        host = rel.RelatingBuildingElement
        if host.is_a("IfcWall") or host.is_a("IfcCurtainWall"):
            continue
        opening = rel.RelatedOpeningElement
        count += 1
        box = "-"
        try:
            shape = ifcopenshell.geom.create_shape(settings, opening)
            verts = np.array(shape.geometry.verts).reshape(-1, 3)
            lo, hi = verts.min(axis=0), verts.max(axis=0)
            box = f"({lo[0]:.4f}, {lo[1]:.4f}, {lo[2]:.4f})..({hi[0]:.4f}, {hi[1]:.4f}, {hi[2]:.4f})"
        except RuntimeError:
            pass
        items = []
        if opening.Representation:
            for rep in opening.Representation.Representations:
                items.extend(describe(i) for i in rep.Items)
        print(
            f"opening {opening.Tag} '{opening.Name}' {opening.GlobalId} in {host.is_a()} {host.Tag}"
            f" ({host.Name}): box {box}; " + " | ".join(items)
        )
    print(f"{count} openings in hosts other than walls")


def main():
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    listing("Revit's export", args[1])
    ours = os.path.join(os.path.dirname(os.path.abspath(args[0])), "model.ifc")
    if os.path.exists(ours):
        listing("rvt-rs export", ours)
    return 0


if __name__ == "__main__":
    sys.exit(main())

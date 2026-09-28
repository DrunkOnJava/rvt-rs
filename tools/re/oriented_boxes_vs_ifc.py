#!/usr/bin/env python3
"""Score curtain mullions and panels drawn along their own axes against
Revit's IFC4.

Research tool for `reports/element-framing/RE-106-oriented-curtain-boxes.md`.
Both files are meshed by IfcOpenShell and moved into Revit's internal feet
through each export's own site placement. For every element of the rvt-rs
file whose `BodySource` is `partition_family_instance_oriented_box`, the
box's axes are read from its `IfcExtrudedAreaSolid` placement (X its
RefDirection, Z its Axis, Y = Z x X), and Revit's mesh for the same Tag is
measured along them: its extent and its centre along each axis, against the
box's.

Usage:

    python3 tools/re/oriented_boxes_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc> [--list N]

Needs IfcOpenShell (tested with 0.8.5) and NumPy.
"""

import collections
import importlib.util
import os
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element
import numpy as np

SOURCE = "partition_family_instance_oriented_box"
MATCH_FEET = 0.01


def main(argv):
    args = argv[1:]
    listing = 0
    if "--list" in args:
        i = args.index("--list")
        listing = int(args[i + 1])
        del args[i : i + 2]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    here = os.path.join(os.path.dirname(os.path.abspath(__file__)), "plan_profiles_vs_ifc.py")
    spec = importlib.util.spec_from_file_location("plan_profiles_vs_ifc", here)
    profiles = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(profiles)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)

    ours_file = ifcopenshell.open(args[0])
    ours_frame = profiles.site_frame(ours_file)
    boxes = {}
    for element in ours_file.by_type("IfcElement"):
        psets = ifcopenshell.util.element.get_psets(element)
        if not any(p.get("BodySource") == SOURCE for p in psets.values()):
            continue
        solid = next(
            item
            for item in ours_file.traverse(element.Representation)
            if item.is_a("IfcExtrudedAreaSolid")
        )
        z = np.array(solid.Position.Axis.DirectionRatios, dtype=float)
        x = np.array(solid.Position.RefDirection.DirectionRatios, dtype=float)
        z /= np.linalg.norm(z)
        x -= z * (x @ z)
        x /= np.linalg.norm(x)
        axes = np.array([x, np.cross(z, x), z])
        shape = ifcopenshell.geom.create_shape(settings, element)
        verts = ours_frame(np.array(shape.geometry.verts, dtype=float).reshape(-1, 3))
        boxes[element.Tag] = (axes, verts)

    revit_file = ifcopenshell.open(args[1])
    revit_frame = profiles.site_frame(revit_file)
    revit = collections.defaultdict(list)
    for element in revit_file.by_type("IfcElement"):
        if element.Tag in boxes and element.Representation:
            shape = ifcopenshell.geom.create_shape(settings, element)
            revit[element.Tag].append(
                revit_frame(np.array(shape.geometry.verts, dtype=float).reshape(-1, 3))
            )

    counts = collections.Counter()
    rows = []
    for tag, (axes, verts) in boxes.items():
        if tag not in revit:
            counts["not in Revit's export"] += 1
            continue
        theirs = np.vstack(revit[tag]) @ axes.T
        ours = verts @ axes.T
        gap = max(
            np.abs(np.ptp(ours, axis=0) - np.ptp(theirs, axis=0)).max(),
            np.abs((ours.max(axis=0) + ours.min(axis=0)) / 2 - (theirs.max(axis=0) + theirs.min(axis=0)) / 2).max(),
        )
        counts["box = Revit's extents along its axes" if gap <= MATCH_FEET else "differs"] += 1
        rows.append((gap, tag))
    print(f"oriented boxes: {len(boxes)}")
    for what, count in counts.most_common():
        print(f"  {what}: {count}")
    for gap, tag in sorted(rows, reverse=True)[:listing]:
        print(f"  {tag}: {gap:.4f} ft")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

#!/usr/bin/env python3
"""List Revit's top-surface outline of each owner `probe_824_slab_arcs`
prints (#824), with rvt-rs's beside it when given.

For each `owner <id>` line of the probe output, the Revit element with that
Tag is meshed (`plan_profiles_vs_ifc.tops`) and the rings of its top
surface printed in Revit internal feet: the exterior and every hole, vertex
by vertex, with the arcs' chords marked by their short edges.

Usage:

    python3 tools/re/slab_outlines_list.py <probe output> <revit.ifc> [--list]
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from plan_profiles_vs_ifc import tops  # noqa: E402


def rings(surface):
    polygons = getattr(surface, "geoms", [surface])
    for polygon in polygons:
        yield "exterior", polygon.exterior.simplify(1e-4).coords
        for hole in polygon.interiors:
            yield "hole", hole.simplify(1e-4).coords


def main():
    owners = [line.split()[1] for line in open(sys.argv[1], encoding="utf-8", errors="replace") if line.startswith("owner ")]
    revit = tops(sys.argv[2], ["IfcSlab", "IfcRoof", "IfcShadingDevice"])
    for tag in owners:
        if tag not in revit:
            print(f"owner {tag}: not in Revit's export")
            continue
        area, _, cls, surface = revit[tag]
        print(f"owner {tag}: {cls} top area {area:.4f} ft2")
        for kind, coords in rings(surface):
            coords = list(coords)
            print(f"  {kind} {len(coords) - 1} vertices")
            for (x0, y0), (x1, y1) in zip(coords, coords[1:]):
                length = ((x1 - x0) ** 2 + (y1 - y0) ** 2) ** 0.5
                print(f"    {x0:.4f} {y0:.4f} -> {x1:.4f} {y1:.4f} ({length:.4f})")


if __name__ == "__main__":
    main()

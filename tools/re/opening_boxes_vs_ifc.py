#!/usr/bin/env python3
"""Score door and window openings against Revit's IFC4, in Revit's internal frame.

Research tool for `reports/element-framing/RE-93-window-openings.md` (#227).
Unlike `openings_vs_ifc.py`, which maps rvt-rs's boxes into Revit's shared
frame through an approximate transform, both files' openings are moved into
Revit's internal feet through each export's own site placement, so the
comparison is exact. Each `IfcOpeningElement` is keyed by the Tag of the
door or window filling it.

For each filler in both files it reports the largest difference between
the two openings' axis-aligned boxes, in feet, bucketed by class, and lists
the worst.

Usage:

    python3 tools/re/opening_boxes_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc> [--list N]

Needs IfcOpenShell (tested with 0.8.5) and NumPy.
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.placement
import numpy as np

FEET = 0.3048


def site_frame(f):
    """Shared metres (IfcOpenShell world) -> Revit internal feet."""
    site = f.by_type("IfcSite")[0]
    mat = ifcopenshell.util.placement.get_local_placement(site.ObjectPlacement)
    ox, oy, oz = mat[0][3], mat[1][3], mat[2][3]
    c, s = mat[0][0], mat[1][0]

    def points(v):
        dx, dy = v[:, 0] - ox, v[:, 1] - oy
        return np.stack(
            [(c * dx + s * dy) / FEET, (-s * dx + c * dy) / FEET, (v[:, 2] - oz) / FEET], axis=1
        )

    return points


def openings(path):
    """Filler Tag -> (box min, box max, filler class) in internal feet."""
    f = ifcopenshell.open(path)
    frame = site_frame(f)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    out = {}
    for rel in f.by_type("IfcRelFillsElement"):
        fill = rel.RelatedBuildingElement
        if not fill.Tag:
            continue
        try:
            shape = ifcopenshell.geom.create_shape(settings, rel.RelatingOpeningElement)
        except Exception:
            continue
        verts = np.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
        if len(verts):
            v = frame(verts)
            out[fill.Tag] = (v.min(axis=0), v.max(axis=0), fill.is_a())
    return out


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
    ours, revit = openings(args[0]), openings(args[1])
    rows = []
    for tag, (rmin, rmax, fill) in revit.items():
        if tag in ours:
            omin, omax, _ = ours[tag]
            diff = float(max(np.abs(omin - rmin).max(), np.abs(omax - rmax).max()))
            rows.append((diff, tag, fill))
    print(f"openings in Revit's export: {len(revit)}; in rvt-rs's: {len(ours)}; both: {len(rows)}")
    buckets = collections.Counter()
    for diff, _, fill in rows:
        band = next(
            (f"within {limit} ft" for limit in (0.001, 0.01, 0.1, 0.26, 1.0) if diff <= limit),
            "over 1 ft",
        )
        buckets[(fill, band)] += 1
    for (fill, band), count in sorted(buckets.items()):
        print(f"  {fill} {band}: {count}")
    for diff, tag, fill in sorted(rows, reverse=True)[:listing]:
        print(f"  {tag} {fill}: {diff:.3f} ft")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

#!/usr/bin/env python3
"""Score the openings in an rvt-rs IFC export against Revit's own IFC4 export.

Research tool for `reports/element-framing/RE-83-openings-through-walls.md`
(#227). Both files are meshed by IfcOpenShell. Each `IfcOpeningElement` is
keyed by the `Tag` of the door or window that fills it
(`IfcRelFillsElement`), which is the element's ElementId in both exports.
For each filler present in both, it reports the largest difference between
the two openings' world-space bounding boxes, in feet.

Revit writes Snowdon Towers in shared coordinates; rvt-rs writes internal
ones. `--shared DEG,DX,DY,DZ` maps rvt-rs's boxes into Revit's frame (a
rotation about Z by DEG, then a translation in feet) before comparing.

Usage:

    python3 tools/re/openings_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc> [--shared DEG,DX,DY,DZ] [--list N]

Needs IfcOpenShell (tested with 0.8.5).
"""

import collections
import math
import sys

import ifcopenshell
import ifcopenshell.geom

FT = 0.3048


def openings(path):
    """Filler Tag -> (world box in feet, filler class, host class)."""
    f = ifcopenshell.open(path)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    out = {}
    for rel in f.by_type("IfcRelFillsElement"):
        op = rel.RelatingOpeningElement
        fill = rel.RelatedBuildingElement
        host = None
        for voids in op.VoidsElements or []:
            host = voids.RelatingBuildingElement
        try:
            shape = ifcopenshell.geom.create_shape(settings, op)
        except Exception:
            continue
        verts = shape.geometry.verts
        pts = [verts[i : i + 3] for i in range(0, len(verts), 3)]
        if not pts or not fill.Tag:
            continue
        box = [min(p[k] for p in pts) / FT for k in range(3)] + [
            max(p[k] for p in pts) / FT for k in range(3)
        ]
        out[fill.Tag] = (box, fill.is_a(), host.is_a() if host else None)
    return out


def to_shared(box, shared):
    deg, dx, dy, dz = shared
    c, s = math.cos(math.radians(deg)), math.sin(math.radians(deg))
    corners = [(x, y) for x in (box[0], box[3]) for y in (box[1], box[4])]
    moved = [(x * c - y * s + dx, x * s + y * c + dy) for x, y in corners]
    return [
        min(p[0] for p in moved),
        min(p[1] for p in moved),
        box[2] + dz,
        max(p[0] for p in moved),
        max(p[1] for p in moved),
        box[5] + dz,
    ]


def main(argv):
    args = [a for a in argv[1:]]
    shared = None
    listing = 0
    if "--shared" in args:
        i = args.index("--shared")
        shared = [float(v) for v in args[i + 1].split(",")]
        del args[i : i + 2]
    if "--list" in args:
        i = args.index("--list")
        listing = int(args[i + 1])
        del args[i : i + 2]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    ours, revit = openings(args[0]), openings(args[1])
    rows = []
    for tag, (rbox, fill, _) in revit.items():
        if tag not in ours:
            continue
        obox, _, ohost = ours[tag]
        if shared:
            obox = to_shared(obox, shared)
        diff = max(abs(a - b) for a, b in zip(rbox, obox))
        rows.append((diff, tag, fill, ohost))
    rows.sort()
    n = len(rows)
    print(f"openings in Revit's export: {len(revit)}; in rvt-rs's: {len(ours)}; both: {n}")
    for limit in (0.01, 0.26, 1.0):
        print(f"  within {limit} ft: {sum(d <= limit for d, *_ in rows)}")
    if rows:
        print(f"  largest difference: {rows[-1][0]:.2f} ft")
    misses = collections.Counter(
        (fill, host, "over 1 ft" if d > 1 else "0.26 to 1 ft")
        for d, _, fill, host in rows
        if d > 0.26
    )
    for (fill, host, band), count in misses.most_common():
        print(f"  over 0.26 ft: {count} {fill} in {host} ({band})")
    for d, tag, fill, host in rows[::-1][:listing]:
        print(f"  {tag} {fill} in {host}: {d:.2f} ft")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

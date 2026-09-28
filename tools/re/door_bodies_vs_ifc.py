#!/usr/bin/env python3
"""Score door bodies against Revit's IFC4, and the wall-centre mirror rule (RE-121).

Research tool for `reports/element-framing/RE-121-door-body-boxes.md`. Both
files' doors are moved into Revit's internal feet through each export's own
site placement (`opening_boxes_vs_ifc.site_frame`) and keyed by Tag. The
host wall is the one rvt-rs's file voids with the door's opening, and the
axis across the wall is that wall box's thinner plan axis.

For each door in both files it reports:

- whether rvt-rs's box is already Revit's body box;
- whether the "mirror" rule gives it: across the wall, keep the side of
  rvt-rs's box nearer the wall's centreline and reflect it about the
  centreline onto the far side;
- where Revit's body sits across the wall: centred on the wall's
  centreline, or off it.

Usage:

    python3 tools/re/door_bodies_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc> [--class IfcWindow] [--tol FEET] [--list N]

`--class` scores another filler class (default `IfcDoor`).

Needs IfcOpenShell (tested with 0.8.5) and NumPy.
"""

import collections
import os
import sys

import ifcopenshell
import ifcopenshell.geom
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from opening_boxes_vs_ifc import site_frame  # noqa: E402

SETTINGS = ifcopenshell.geom.settings()
SETTINGS.set("use-world-coords", True)


def box(frame, element):
    try:
        shape = ifcopenshell.geom.create_shape(SETTINGS, element)
    except Exception:
        return None
    verts = np.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
    if not len(verts):
        return None
    v = frame(verts)
    return np.concatenate([v.min(axis=0), v.max(axis=0)])


def doors(path, with_hosts, cls):
    """Filler Tag -> (door box, host wall box or None), in internal feet."""
    f = ifcopenshell.open(path)
    frame = site_frame(f)
    host = {}
    if with_hosts:
        voids = {r.RelatedOpeningElement.id(): r.RelatingBuildingElement for r in f.by_type("IfcRelVoidsElement")}
        for rel in f.by_type("IfcRelFillsElement"):
            wall = voids.get(rel.RelatingOpeningElement.id())
            if wall is not None:
                host[rel.RelatedBuildingElement.id()] = wall
    out = {}
    for door in f.by_type(cls):
        if not door.Tag:
            continue
        b = box(frame, door)
        if b is None:
            continue
        wall = host.get(door.id())
        out[door.Tag] = (b, box(frame, wall) if wall is not None else None)
    return out


def main(argv):
    args = argv[1:]
    tol, listing, cls = 0.01, 0, "IfcDoor"
    for flag in ("--tol", "--list", "--class"):
        if flag in args:
            i = args.index(flag)
            value = args[i + 1]
            del args[i : i + 2]
            if flag == "--tol":
                tol = float(value)
            elif flag == "--class":
                cls = value
            else:
                listing = int(value)
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    ours, ref = doors(args[0], True, cls), doors(args[1], False, cls)
    counts, placement, rows = collections.Counter(), collections.Counter(), []
    for tag, (b, wall) in sorted(ours.items()):
        if tag not in ref:
            counts["not in Revit's export"] += 1
            continue
        r = ref[tag][0]
        if wall is None:
            counts["no host wall in rvt-rs's file"] += 1
            continue
        wx, wy = wall[3] - wall[0], wall[4] - wall[1]
        if abs(wx - wy) < 1e-6:
            counts["square host wall box"] += 1
            continue
        ax = 0 if wx < wy else 1
        centre = (wall[ax] + wall[ax + 3]) / 2
        offset = (r[ax] + r[ax + 3]) / 2 - centre
        placement["Revit's body centred on the wall" if abs(offset) < tol else "Revit's body off the wall's centre"] += 1
        if np.all(np.abs(b - r) < tol):
            counts["rvt-rs's box is Revit's"] += 1
            continue
        mirrored = b.copy()
        if centre - b[ax] < b[ax + 3] - centre:
            mirrored[ax + 3] = 2 * centre - b[ax]
        else:
            mirrored[ax] = 2 * centre - b[ax + 3]
        if np.all(np.abs(mirrored - r) < tol):
            counts["mirror rule gives Revit's"] += 1
        else:
            counts["mirror rule wrong"] += 1
            rows.append((tag, round(offset, 3), np.round(b - r, 3), np.round(mirrored - r, 3)))
    print(f"{len(ours)} {cls} in rvt-rs's file; tolerance {tol} ft")
    for key, value in sorted(counts.items()):
        print(f"  {value:5d} {key}")
    for key, value in sorted(placement.items()):
        print(f"  {value:5d} {key}")
    for row in rows[:listing]:
        print("  Tag %s, Revit centre off the wall's by %s ft; box - Revit %s; mirrored - Revit %s" % row)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

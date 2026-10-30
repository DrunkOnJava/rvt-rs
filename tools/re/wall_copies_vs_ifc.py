#!/usr/bin/env python3
"""Compare each copy of a wall's location line with Revit's wall (#824).

Reads `probes/examples/probe_824_wall_copies.rs`'s output. For each wall whose
copies disagree, it meshes Revit's IfcWall of the same Tag in the site frame
(Revit internal feet) and prints, per copy, how far the copy's two ends lie
from the ends of Revit's body along the line, and the body's distance from
the line across it, marking the current copy.

Usage:

    python3 tools/re/wall_copies_vs_ifc.py <probe output> <revit.ifc> [--list]
"""

import collections
import math
import os
import sys

import ifcopenshell
import ifcopenshell.geom
import numpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from opening_boxes_vs_ifc import site_frame  # noqa: E402


def main():
    current, copies = {}, collections.defaultdict(list)
    for line in open(sys.argv[1], encoding="utf-8", errors="replace"):
        parts = line.split()
        if parts[:1] == ["wall"] and len(parts) == 4:
            current[parts[1]] = parts[3]
        elif parts[:1] == ["copy"] and len(parts) == 9:
            copies[parts[1]].append((parts[2], [float(v) for v in parts[3:9]]))
    model = ifcopenshell.open(sys.argv[2])
    frame = site_frame(model)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    walls = {str(w.Tag): w for w in model.by_type("IfcWall") if w.Tag}
    tally = collections.Counter()
    print(f"walls with disagreeing copies: {len(copies)}")
    for tag in sorted(copies, key=int):
        wall = walls.get(tag)
        if wall is None:
            print(f"wall {tag}: not in Revit's export")
            continue
        try:
            shape = ifcopenshell.geom.create_shape(settings, wall)
        except RuntimeError:
            print(f"wall {tag}: no body")
            continue
        verts = frame(numpy.array(shape.geometry.verts, dtype=float).reshape(-1, 3))
        print(f"wall {tag}: current {current.get(tag)}")
        best = None
        for partition, (sx, sy, sz, ex, ey, ez) in copies[tag]:
            length = math.hypot(ex - sx, ey - sy)
            if length <= 0:
                continue
            d = ((ex - sx) / length, (ey - sy) / length)
            along = (verts[:, 0] - sx) * d[0] + (verts[:, 1] - sy) * d[1]
            lo, hi = float(along.min()), float(along.max())
            miss = max(abs(lo - 0.0), abs(hi - length))
            mark = " (current)" if partition == current.get(tag) else ""
            print(f"  copy in Partitions/{partition}{mark}: length {length:.4f}, Revit's body from {lo:.4f} to {hi:.4f} along it, ends off by {miss:.4f} ft")
            if best is None or miss < best[0]:
                best = (miss, partition)
        if best:
            tally["current copy nearest" if best[1] == current.get(tag) else "another copy nearest"] += 1
    print(dict(tally))


if __name__ == "__main__":
    main()

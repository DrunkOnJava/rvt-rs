#!/usr/bin/env python3
"""Whether a stair landing's own data holds the outline Revit's export draws
(RE-210, #824).

Takes the output of `examples/probe_re210_landing_outlines.rs` (every bounded
line and arc in each landing's data) and Revit's export. For each landing it
reports Revit's top-surface outline (from `plan_profiles_vs_ifc.tops`, in
Revit's internal feet), how far each of Revit's outline vertices is from the
nearest end of the landing's lines and arcs, whether the lines at the top's
elevation close into a loop, and that loop's area against Revit's.

Needs IfcOpenShell and Shapely.

Usage:

    python3 tools/re/landing_outlines_vs_ifc.py <probe.txt> <revit.ifc> [--list]
"""

import argparse
import collections
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from plan_profiles_vs_ifc import tops  # noqa: E402

from shapely.geometry import Polygon  # noqa: E402


def read_probe(path):
    boxes, lines, arcs = {}, collections.defaultdict(list), collections.defaultdict(list)
    for raw in open(path, encoding="utf-8", errors="replace"):
        p = raw.split()
        if not p:
            continue
        if p[0] == "landing":
            boxes[p[1]] = [float(v) for v in p[3:9]]
        elif p[0] == "line":
            v = [float(x) for x in p[2:8]]
            lines[p[1]].append(((v[0], v[1], v[2]), (v[3], v[4], v[5])))
        elif p[0] == "arc":
            v = [float(x) for x in p[2:12]]
            arcs[p[1]].append(v)
    return boxes, lines, arcs


def arc_ends(a):
    cx, cy, cz, r, s, e, xx, xy, yx, yy = a
    out = []
    for t in (s, e):
        out.append((cx + r * (math.cos(t) * xx + math.sin(t) * yx), cy + r * (math.cos(t) * xy + math.sin(t) * yy), cz))
    return out


def loop_area(segments):
    """The area of the loop the segments make, end to end, or None."""
    if len(segments) < 3:
        return None
    remaining = list(segments)
    first = remaining.pop(0)
    loop, here = [first[0][:2], first[1][:2]], first[1][:2]
    while remaining:
        nxt = next((s for s in remaining if math.dist(s[0][:2], here) < 1e-3 or math.dist(s[1][:2], here) < 1e-3), None)
        if nxt is None:
            return None
        remaining.remove(nxt)
        here = nxt[1][:2] if math.dist(nxt[0][:2], here) < 1e-3 else nxt[0][:2]
        loop.append(here)
    if math.dist(loop[0], loop[-1]) > 1e-3:
        return None
    return Polygon(loop[:-1]).area


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("probe")
    ap.add_argument("reference")
    ap.add_argument("--list", action="store_true")
    args = ap.parse_args()
    boxes, lines, arcs = read_probe(args.probe)
    revit = tops(args.reference, ["IfcSlab", "IfcStair", "IfcStairFlight", "IfcBuildingElementProxy", "IfcCovering"])
    print(f"landings: {len(boxes)}, with lines: {len(lines)}, with arcs: {len(arcs)}, in Revit's export: {sum(1 for t in boxes if t in revit)}")
    summary = collections.Counter()
    for tag, box in sorted(boxes.items()):
        if tag not in revit:
            summary["not in Revit's export"] += 1
            continue
        area, points, cls, surface = revit[tag]
        ends = [e for seg in lines.get(tag, []) for e in seg] + [e for a in arcs.get(tag, []) for e in arc_ends(a)]
        if not ends:
            summary["no lines or arcs"] += 1
            print(f"  {tag} {cls}: Revit {area:.2f} ft2, nothing in the data")
            continue
        exterior = list(surface.exterior.coords) if hasattr(surface, "exterior") else []
        worst = max((min(math.dist(v[:2], e[:2]) for e in ends) for v in exterior), default=float("nan"))
        top_z = max(e[2] for e in ends)
        level_segments = [s for s in lines.get(tag, []) if abs(s[0][2] - top_z) < 1e-3 and abs(s[1][2] - top_z) < 1e-3]
        ours = loop_area(level_segments)
        verdict = "vertices within 0.01 ft" if worst <= 0.01 else "vertices off"
        summary[verdict] += 1
        if args.list or worst > 0.01:
            print(
                f"  {tag} {cls}: Revit {area:.2f} ft2, {len(exterior) - 1} vertices; data {len(lines.get(tag, []))} lines, "
                f"{len(arcs.get(tag, []))} arcs; worst vertex {worst:.4f} ft; loop at the top {'none' if ours is None else f'{ours:.2f} ft2'}"
            )
    print("summary: " + ", ".join(f"{k} {v}" for k, v in summary.most_common()))


if __name__ == "__main__":
    main()

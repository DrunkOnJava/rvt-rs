#!/usr/bin/env python3
"""A roof by extrusion's sketch curves and data against Revit's body (RE-211,
#883).

Takes the output of `examples/probe_re211_extrusion_roofs.rs` and Revit's
IFC export. For every roof the probe lists that Revit's export holds (same
Tag), this meshes Revit's body in world space with IfcOpenShell and reports:

- the body's vertices: how many, their box, and how many distinct values
  each axis takes (an extrusion along a model axis takes two along it);
- each vertex's distance, in the plane across that axis, to the nearest of
  the roof's sketch curves (lines and arcs whose record names the roof), as
  a histogram: a profile drawn on the sketch puts its vertices at 0 and at
  the roof's thickness;
- which f64 of the roof's data equals the body's start or end along the
  axis, or their distance from the sketch's plane.

Needs IfcOpenShell (`pip install ifcopenshell`).

Usage:

    python3 tools/re/extrusion_roofs_vs_ifc.py <probe.txt> <revit.ifc> [--list]
"""

import argparse
import collections
import math
import re

import ifcopenshell
import ifcopenshell.geom

from roof_bodies_vs_ifc import shapes_of

FT = 0.3048
MATCH_FT = 1e-4


def floats(text):
    return [float(v) for v in text.split(",")]


def read_probe(path):
    roofs, curves, values = {}, collections.defaultdict(list), collections.defaultdict(list)
    for line in open(path, encoding="utf-8", errors="replace"):
        parts = line.split()
        if not parts:
            continue
        if parts[0] == "roof":
            roofs[int(parts[1])] = {"class": parts[2], "box": floats(parts[3].split("=", 1)[1])}
        elif parts[0] == "line" and "roof" in parts:
            owner = int(parts[2].split("=", 1)[1])
            fields = dict(p.split("=", 1) for p in parts[3:] if "=" in p)
            m = re.match(r"([-\d.e,]+),r=([-\d.e]+),x=([-\d.e,]+),y=([-\d.e,]+),a=([-\d.e]+)\.\.([-\d.e]+)$", line.split("arc=", 1)[1].strip())
            arc = None
            if m:
                arc = {
                    "centre": floats(m.group(1)),
                    "radius": float(m.group(2)),
                    "x": floats(m.group(3)),
                    "y": floats(m.group(4)),
                    "angles": (float(m.group(5)), float(m.group(6))),
                }
            seg = None
            if fields.get("line", "-") != "-":
                a, b = fields["line"].split(":")
                seg = (floats(a), floats(b))
            curves[owner].append({"id": int(parts[1]), "box": floats(fields["box"]), "line": seg, "arc": arc})
        elif parts[0] == "f64":
            values[int(parts[1])].append((parts[2], float(parts[3])))
    return roofs, curves, values


def body_vertices(model, element):
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    out = []
    for shape_of in shapes_of(element):
        shape = ifcopenshell.geom.create_shape(settings, shape_of)
        v = shape.geometry.verts
        out += [tuple(v[i + k] / FT for k in range(3)) for i in range(0, len(v), 3)]
    return out


def in_box(point, box, slack=1e-3):
    return all(box[k] - slack <= point[k] <= box[k + 3] + slack for k in range(3))


def distance_to_curve(p, curve, axis):
    keep = [k for k in range(3) if k != axis]
    best = math.inf
    if curve["line"]:
        a, b = curve["line"]
        ax, ay = a[keep[0]], a[keep[1]]
        bx, by = b[keep[0]], b[keep[1]]
        px, py = p[keep[0]], p[keep[1]]
        dx, dy = bx - ax, by - ay
        length2 = dx * dx + dy * dy
        t = 0.0 if length2 == 0 else max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / length2))
        best = min(best, math.hypot(px - ax - t * dx, py - ay - t * dy))
    if curve["arc"] and in_box(curve["arc"]["centre"], curve["box"], slack=math.inf):
        c = curve["arc"]["centre"]
        r = curve["arc"]["radius"]
        best = min(best, abs(math.hypot(p[keep[0]] - c[keep[0]], p[keep[1]] - c[keep[1]]) - r))
    return best


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("probe")
    ap.add_argument("reference")
    ap.add_argument("--list", action="store_true")
    args = ap.parse_args()
    roofs, curves, values = read_probe(args.probe)
    print(f"roofs in the probe: {len(roofs)}, with sketch curves: {sum(1 for r in roofs if curves.get(r))}")
    model = ifcopenshell.open(args.reference)
    revit = {}
    for roof in model.by_type("IfcRoof"):
        tag = str(getattr(roof, "Tag", "") or "")
        if tag.isdigit():
            revit[int(tag)] = roof
    for roof_id, roof in sorted(roofs.items()):
        print(f"roof {roof_id} {roof['class']} box {', '.join(f'{v:.4f}' for v in roof['box'])}")
        element = revit.get(roof_id)
        if element is None:
            print("  not in Revit's export")
            continue
        vertices = body_vertices(model, element)
        if not vertices:
            print("  Revit's body has no vertices")
            continue
        lows = [min(v[k] for v in vertices) for k in range(3)]
        highs = [max(v[k] for v in vertices) for k in range(3)]
        distinct = [len({round(v[k], 4) for v in vertices}) for k in range(3)]
        print(f"  Revit's body: {len(vertices)} vertices, box {', '.join(f'{v:.4f}' for v in lows + highs)}, distinct per axis {distinct}")
        axis = min(range(3), key=lambda k: distinct[k])
        mine = curves.get(roof_id, [])
        print(f"  extrusion axis {'xyz'[axis]} from {lows[axis]:.6f} to {highs[axis]:.6f}; sketch curves {len(mine)}: "
              f"{sum(1 for c in mine if c['line'])} read as lines, {sum(1 for c in mine if c['arc'])} as arcs")
        for c in mine:
            plane = (c["box"][axis], c["box"][axis + 3])
            print(f"    curve {c['id']} box along {'xyz'[axis]} {plane[0]:.6f}..{plane[1]:.6f}"
                  + (f" arc r {c['arc']['radius']:.6f} centre {', '.join(f'{v:.4f}' for v in c['arc']['centre'])}" if c["arc"] else "")
                  + (" line" if c["line"] else ""))
        if mine:
            distances = [min(distance_to_curve(v, c, axis) for c in mine) for v in vertices]
            hist = collections.Counter(round(d, 4) for d in distances)
            print("  vertex distance to the sketch, feet: " + ", ".join(f"{d:.4f} x{n}" for d, n in hist.most_common(6)))
            print(f"  worst {max(distances):.6f}, on the sketch (< {MATCH_FT} ft) {sum(1 for d in distances if d < MATCH_FT)} of {len(distances)}")
            sketch_plane = mine[0]["box"][axis]
        else:
            sketch_plane = None
        targets = {"start": lows[axis], "end": highs[axis]}
        if sketch_plane is not None:
            targets["start - sketch plane"] = lows[axis] - sketch_plane
            targets["end - sketch plane"] = highs[axis] - sketch_plane
            targets["sketch plane - start"] = sketch_plane - lows[axis]
        hits = [(name, off, v) for name, t in targets.items() for off, v in values.get(roof_id, []) if abs(v - t) < MATCH_FT]
        for name, off, v in hits[: 20 if args.list else 8]:
            print(f"  data {off} = {v:.9f} = {name}")
        if not hits:
            print("  no f64 of the roof's data is the body's start or end")


if __name__ == "__main__":
    main()

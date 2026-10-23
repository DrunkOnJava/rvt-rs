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
  axis, or their distance from the sketch's plane;
- for a sketch curve whose data holds a B-spline (after the bounded-curve
  tag: start and end parameter, u32 degree, two flag bytes, u32 count, that
  many weight and x, y, z, then a u32-counted knot list), the section that
  curve gives as the roof's top, its offset square to it by the thickness
  the record box's bottom implies, and plumb cuts at its two ends, extruded
  over the body's extent: every Revit vertex's distance from that solid's
  boundary, and the two volumes.

Needs IfcOpenShell (`pip install ifcopenshell`).

Usage:

    python3 tools/re/extrusion_roofs_vs_ifc.py <probe.txt> <revit.ifc> [--list]
"""

import argparse
import collections
import math
import re
import struct

import ifcopenshell
import ifcopenshell.geom

from roof_bodies_vs_ifc import shapes_of

FT = 0.3048
MATCH_FT = 1e-4


def floats(text):
    return [float(v) for v in text.split(",")]


def read_probe(path):
    roofs, curves, values = {}, collections.defaultdict(list), collections.defaultdict(list)
    data = {}
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
        elif parts[0] == "data" and len(parts) >= 5:
            data.setdefault(int(parts[1]), bytes.fromhex(parts[4]))
    return roofs, curves, values, data


def body_mesh(model, element):
    """Revit's body in world feet: its vertices and its triangles' volume."""
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    out, volume = [], 0.0
    for shape_of in shapes_of(element):
        shape = ifcopenshell.geom.create_shape(settings, shape_of)
        v = shape.geometry.verts
        points = [tuple(v[i + k] / FT for k in range(3)) for i in range(0, len(v), 3)]
        f = shape.geometry.faces
        for i in range(0, len(f), 3):
            a, b, c = (points[f[i + k]] for k in range(3))
            volume += (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0]) + a[2] * (b[0] * c[1] - b[1] * c[0])) / 6
        out += points
    return out, abs(volume)


TAG = bytes.fromhex("04000801")


def spline_at(data, at):
    """The B-spline whose bounded-curve tag is at `at`, or None."""
    try:
        start, end = struct.unpack_from("<2d", data, at + 4)
        degree, = struct.unpack_from("<I", data, at + 20)
        count, = struct.unpack_from("<I", data, at + 26)
        if not (1 <= degree <= 9 and degree < count <= 1000):
            return None
        points = [struct.unpack_from("<4d", data, at + 30 + 32 * i) for i in range(count)]
        knots_at = at + 30 + 32 * count
        n_knots, = struct.unpack_from("<I", data, knots_at)
        if n_knots != count + degree + 1:
            return None
        knots = struct.unpack_from(f"<{n_knots}d", data, knots_at + 4)
    except struct.error:
        return None
    if any(b < a for a, b in zip(knots, knots[1:])) or any(w <= 0 for w, *_ in points):
        return None
    return {"start": start, "end": end, "degree": degree, "points": points, "knots": knots}


def spline_point(spline, u):
    """The point at parameter `u` (Cox-de Boor, rational)."""
    knots, p, pts = spline["knots"], spline["degree"], spline["points"]
    u = min(max(u, knots[p]), knots[-p - 1])
    span = max(i for i in range(p, len(pts)) if knots[i] <= u) if u < knots[-p - 1] else len(pts) - 1
    n = [0.0] * (p + 1)
    n[0] = 1.0
    left, right = [0.0] * (p + 1), [0.0] * (p + 1)
    for j in range(1, p + 1):
        left[j] = u - knots[span + 1 - j]
        right[j] = knots[span + j] - u
        saved = 0.0
        for r in range(j):
            temp = n[r] / (right[r + 1] + left[j - r])
            n[r] = saved + right[r + 1] * temp
            saved = left[j - r] * temp
        n[j] = saved
    w = sum(n[k] * pts[span - p + k][0] for k in range(p + 1))
    return [sum(n[k] * pts[span - p + k][0] * pts[span - p + k][c + 1] for k in range(p + 1)) / w for c in range(3)]


def section(top, thickness):
    """The closed outline below `top` (a list of (u, v) points along u, v
    up): `top`, plumb cuts at its ends, and its offset square to it by
    `thickness`, each end of the offset carried along its tangent to the
    plumb line."""
    def normal(i):
        a, b = top[max(i - 1, 0)], top[min(i + 1, len(top) - 1)]
        du, dv = b[0] - a[0], b[1] - a[1]
        length = math.hypot(du, dv)
        return (dv / length, -du / length)
    bottom = [(u + thickness * normal(i)[0], v + thickness * normal(i)[1]) for i, (u, v) in enumerate(top)]
    # Carry both ends along their tangents past the plumb lines, then keep
    # what lies between them.
    def carried(a, b):
        return (b[0] + 1e3 * (b[0] - a[0]), b[1] + 1e3 * (b[1] - a[1]))
    bottom = [carried(bottom[1], bottom[0])] + bottom + [carried(bottom[-2], bottom[-1])]
    lo, hi = top[0][0], top[-1][0]
    clipped = []
    for (u0, v0), (u1, v1) in zip(bottom, bottom[1:]):
        for edge in (lo, hi):
            if (u0 - edge) * (u1 - edge) < 0:
                clipped.append((edge, v0 + (v1 - v0) * (edge - u0) / (u1 - u0)))
        if lo <= u1 <= hi:
            clipped.append((u1, v1))
    return top + sorted(clipped, key=lambda p: p[0])[::-1]


def distance_to_outline(point, outline):
    best = math.inf
    for (u0, v0), (u1, v1) in zip(outline, outline[1:] + outline[:1]):
        du, dv = u1 - u0, v1 - v0
        length2 = du * du + dv * dv
        t = 0.0 if length2 == 0 else max(0.0, min(1.0, ((point[0] - u0) * du + (point[1] - v0) * dv) / length2))
        best = min(best, math.hypot(point[0] - u0 - t * du, point[1] - v0 - t * dv))
    return best


def area(outline):
    return abs(sum(u0 * v1 - u1 * v0 for (u0, v0), (u1, v1) in zip(outline, outline[1:] + outline[:1]))) / 2


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
    roofs, curves, values, data = read_probe(args.probe)
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
        vertices, revit_volume = body_mesh(model, element)
        if not vertices:
            print("  Revit's body has no vertices")
            continue
        lows = [min(v[k] for v in vertices) for k in range(3)]
        highs = [max(v[k] for v in vertices) for k in range(3)]
        distinct = [len({round(v[k], 4) for v in vertices}) for k in range(3)]
        print(f"  Revit's body: {len(vertices)} vertices, box {', '.join(f'{v:.4f}' for v in lows + highs)}, distinct per axis {distinct}")
        mine = curves.get(roof_id, [])
        # The sketch's plane is where its curves' boxes are flat; without one,
        # the axis with the fewest distinct values.
        flat = [k for k in range(3) if mine and all(abs(c["box"][k + 3] - c["box"][k]) < 1e-6 for c in mine)]
        axis = flat[0] if len(flat) == 1 else min(range(3), key=lambda k: distinct[k])
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
        splines = []
        for c in mine:
            buf = data.get(c["id"], b"")
            at = buf.find(TAG)
            while at >= 0:
                spline = spline_at(buf, at)
                if spline:
                    splines.append((c["id"], at, spline))
                    break
                at = buf.find(TAG, at + 1)
        for curve_id, at, spline in splines:
            print(f"  curve {curve_id}: B-spline at +{at}, degree {spline['degree']}, {len(spline['points'])} control points, "
                  f"weights {sorted({round(w, 6) for w, *_ in spline['points']})}, knots {[round(k, 4) for k in spline['knots']]}, "
                  f"parameters {spline['start']:.6f}..{spline['end']:.6f}")
        if len(splines) != 1 or len(mine) != 1:
            continue
        spline = splines[0][2]
        keep = [k for k in range(3) if k != axis]
        # The plane's other two axes: u along the curve's run, v the other.
        samples = [spline_point(spline, spline["start"] + (spline["end"] - spline["start"]) * i / 4000) for i in range(4001)]
        spans = [max(s[k] for s in samples) - min(s[k] for s in samples) for k in keep]
        u_axis, v_axis = (keep[0], keep[1]) if spans[0] >= spans[1] else (keep[1], keep[0])
        top = [(s[u_axis], s[v_axis]) for s in samples]
        if top[-1][0] < top[0][0]:
            top = top[::-1]
        monotone = all(b[0] > a[0] for a, b in zip(top, top[1:]))
        box = roof["box"]
        slope0 = math.atan2(top[1][1] - top[0][1], top[1][0] - top[0][0])
        thickness = (top[0][1] - box[v_axis]) * math.cos(slope0)
        outline = section(top, thickness)
        print(f"  section: curve monotone along {'xyz'[u_axis]} {monotone}, top {min(v for _, v in top):.6f}..{max(v for _, v in top):.6f}, "
              f"thickness from the record box's bottom and a plumb cut {thickness:.6f} ft")
        out_box = [min(u for u, _ in outline), min(v for _, v in outline), max(u for u, _ in outline), max(v for _, v in outline)]
        print(f"  section box along {'xyz'[u_axis]} {out_box[0]:.6f}..{out_box[2]:.6f}, along {'xyz'[v_axis]} {out_box[1]:.6f}..{out_box[3]:.6f}; "
              f"record box {box[u_axis]:.6f}..{box[u_axis + 3]:.6f}, {box[v_axis]:.6f}..{box[v_axis + 3]:.6f}")
        length = highs[axis] - lows[axis]
        mine_volume = area(outline) * length
        print(f"  volume: the section over {length:.6f} ft {mine_volume:.4f} ft3, Revit's body {revit_volume:.4f} ft3, "
              f"ratio {mine_volume / revit_volume:.6f}")
        off = [distance_to_outline((v[u_axis], v[v_axis]), outline) for v in vertices]
        for tol in (1e-4, 1e-3, 1e-2):
            print(f"  Revit's vertices within {tol} ft of the section's outline: {sum(1 for d in off if d < tol)} of {len(off)}")
        print(f"  worst {max(off):.6f} ft")


if __name__ == "__main__":
    main()

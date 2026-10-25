#!/usr/bin/env python3
"""Whether a stair landing's own data holds the outline Revit's export draws
(RE-210, #824).

Takes the output of `examples/probe_re210_landing_outlines.rs` (every bounded
line and arc in each landing's data) and Revit's export. For each landing it
reports Revit's top-surface outline (from `plan_profiles_vs_ifc.tops`, in
Revit's internal feet), and, for each elevation the landing's lines lie at,
the plan ring they make end to end, a gap where one chain stops bridged by a
straight line to the nearest end of an unused line: its area and its
Hausdorff distance from Revit's outline. The elevation whose ring is
closest is reported.

It then compares each landing's body heights: Revit's body's lowest and
highest point, against the highest elevation the landing's lines lie at and
the members of its landing type, which its stairs' type names
(`StairsLanding.m_stairsId` -> `StairsElement.m_typeId` ->
`StairsType.m_landingTypeId`, the `native` lines).

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
from plan_profiles_vs_ifc import site_frame, tops  # noqa: E402

import ifcopenshell  # noqa: E402
import ifcopenshell.geom  # noqa: E402
import numpy as np  # noqa: E402

from shapely.geometry import Polygon  # noqa: E402


def read_probe(path):
    boxes, lines, arcs = {}, collections.defaultdict(list), collections.defaultdict(list)
    native = collections.defaultdict(dict)
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
        elif p[0] == "native" and len(p) >= 4 and p[2] == "0":
            members = dict(m.split("=", 1) for m in p[4:] if "=" in m)
            native[p[1]] = {"class": p[3].split("/")[0], **members}
    return boxes, lines, arcs, native


def heights(path, wanted):
    """Tag -> (lowest, highest) z of Revit's body, internal feet."""
    f = ifcopenshell.open(path)
    frame = site_frame(f)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    out = {}
    for element in f.by_type("IfcSlab"):
        tag = getattr(element, "Tag", None)
        if tag not in wanted or not element.Representation:
            continue
        try:
            shape = ifcopenshell.geom.create_shape(settings, element)
        except Exception:
            continue
        verts = np.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
        if not len(verts):
            continue
        v = frame(verts)
        z = v[:, 2]
        low, high = out.get(tag, (float("inf"), float("-inf")))
        out[tag] = (min(low, float(z.min())), max(high, float(z.max())))
        if tag in SHOWN:
            faces = np.array(shape.geometry.faces, dtype=int).reshape(-1, 3)
            print(f"revit {tag} vertices {len(v)} triangles {len(faces)}")
            for x, y, h in np.unique(np.round(v, 4), axis=0):
                print(f"  vertex {tag} {x:.4f} {y:.4f} {h:.4f}")
    return out


# Landings whose whole body is printed: one of each storey height, the
# landing with another riser height and the one that is not monolithic.
SHOWN = {"2859208", "2861138", "3202702", "3058084"}


def vertical(boxes, lines, native, path, args_list=True):
    revit = heights(path, set(boxes))
    summary = collections.Counter()
    print("body heights:")
    for tag in sorted(boxes):
        if tag not in revit:
            continue
        low, high = revit[tag]
        levels = [s[0][2] for s in lines.get(tag, []) if abs(s[0][2] - s[1][2]) < 1e-3 and abs(s[0][2]) > 1e-3]
        top = max(levels) if levels else None
        record = native.get(tag, {})
        # landing -> its stairs -> the stairs' type -> that type's landing type
        stairs = native.get(record.get("m_stairsId", "@")[1:], {})
        stairs_type = native.get(stairs.get("m_typeId", "@")[1:], {})
        landing_type = stairs_type.get("m_landingTypeId", "@")[1:]
        types = [landing_type] if landing_type in native else []
        kind = native.get(landing_type, {})
        thickness = float(kind["m_thickness"]) if "m_thickness" in kind else None
        top_ok = top is not None and abs(top - high) <= 1e-3
        thick_ok = thickness is not None and abs(thickness - (high - low)) <= 1e-3
        summary[f"top {'=' if top_ok else '!='} lines' elevation"] += 1
        summary[f"thickness {'=' if thick_ok else '!='} type m_thickness"] += 1
        print(
            f"  {tag}: Revit z {low:.4f} to {high:.4f} ({high - low:.4f} ft); lines' highest z {top}; "
            f"box z {boxes[tag][2]:.4f} to {boxes[tag][2] + boxes[tag][5]:.4f}; type {types} "
            + " ".join(f"{k}={v}" for k, v in sorted(kind.items()) if k != "class" and not v.startswith("@"))
        )
        if args_list:
            print("    stairs: " + " ".join(f"{k}={v}" for k, v in sorted(stairs.items()) if not v.startswith("@")))
            print("    stairs type: " + " ".join(f"{k}={v}" for k, v in sorted(stairs_type.items()) if not v.startswith("@")))
    print("summary: " + ", ".join(f"{k} {v}" for k, v in sorted(summary.items())))


def arc_ends(a):
    cx, cy, cz, r, s, e, xx, xy, yx, yy = a
    out = []
    for t in (s, e):
        out.append((cx + r * (math.cos(t) * xx + math.sin(t) * yx), cy + r * (math.cos(t) * xy + math.sin(t) * yy), cz))
    return out


def ring(segments):
    """The closed plan ring the segments make end to end, with the number of
    gaps bridged by a straight line from where one chain stops to the
    nearest end of an unused segment; None for fewer than three segments."""
    if len(segments) < 3:
        return None
    remaining = [(s[0][:2], s[1][:2]) for s in segments]
    a, b = remaining.pop(0)
    points, here, gaps = [a, b], b, 0
    while remaining:
        best = min(
            ((math.dist(seg[0], here), i, False) for i, seg in enumerate(remaining)),
            default=None,
        )
        best_rev = min(((math.dist(seg[1], here), i, True) for i, seg in enumerate(remaining)), default=None)
        distance, index, reverse = min(best, best_rev)
        seg = remaining.pop(index)
        near, far = (seg[1], seg[0]) if reverse else (seg[0], seg[1])
        if distance > 1e-3:
            gaps += 1
            points.append(near)
        points.append(far)
        here = far
    if math.dist(points[0], here) > 1e-3:
        gaps += 1
    else:
        points.pop()
    return Polygon(points), gaps


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("probe")
    ap.add_argument("reference")
    ap.add_argument("--list", action="store_true")
    args = ap.parse_args()
    boxes, lines, arcs, native = read_probe(args.probe)
    revit = tops(args.reference, ["IfcSlab", "IfcStair", "IfcStairFlight", "IfcBuildingElementProxy", "IfcCovering"])
    print(f"landings: {len(boxes)}, with lines: {len(lines)}, with arcs: {len(arcs)}, in Revit's export: {sum(1 for t in boxes if t in revit)}")
    summary = collections.Counter()
    for tag, box in sorted(boxes.items()):
        if tag not in revit:
            summary["not in Revit's export"] += 1
            continue
        area, points, cls, surface = revit[tag]
        if not lines.get(tag):
            summary["no lines"] += 1
            print(f"  {tag} {cls}: Revit {area:.2f} ft2, no lines in the data")
            continue
        by_z = collections.defaultdict(list)
        for seg in lines[tag]:
            if abs(seg[0][2] - seg[1][2]) < 1e-3:
                by_z[round(seg[0][2], 3)].append(seg)
        best = None
        for z, segments in sorted(by_z.items()):
            made = ring(segments)
            if made is None:
                continue
            polygon, gaps = made
            if not polygon.is_valid:
                continue
            distance = polygon.hausdorff_distance(surface)
            if best is None or distance < best[0]:
                best = (distance, z, len(segments), gaps, polygon.area)
        if best is None:
            summary["no ring"] += 1
            print(f"  {tag} {cls}: Revit {area:.2f} ft2; no group of lines makes a ring")
            continue
        distance, z, count, gaps, ours = best
        band = next((f"within {b} ft" for b in (0.001, 0.01, 0.1) if distance <= b), "over 0.1 ft")
        summary[f"outline {band}, {'closed' if gaps == 0 else 'gaps bridged'}"] += 1
        if args.list or distance > 0.01:
            print(
                f"  {tag} {cls}: Revit {area:.2f} ft2; ring of {count} lines at z {z} with {gaps} gaps bridged, "
                f"{ours:.2f} ft2, outline {distance:.4f} ft from Revit's"
            )
    print("summary: " + ", ".join(f"{k} {v}" for k, v in summary.most_common()))
    vertical(boxes, lines, native, args.reference)


if __name__ == "__main__":
    main()

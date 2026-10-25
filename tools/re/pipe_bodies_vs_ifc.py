#!/usr/bin/env python3
"""Score rvt-rs pipe bodies against Revit's own IFC export, per pipe (#96).

Research tool for `reports/element-framing/RE-131-pipe-cylinders.md`. Revit
writes every pipe as an `IfcExtrudedAreaSolid` over an `IfcCircleProfileDef`.
For each pipe both exports hold (same `Tag`, the ElementId), this compares the
profile, its radius, the two ends of the extrusion axis and the extrusion's
length, all in feet. The world box is not compared: for a pipe along a
principal axis the box body and the cylinder share it, so it cannot tell them
apart.

Needs IfcOpenShell (`pip install ifcopenshell`).

Usage:

    python3 tools/re/pipe_bodies_vs_ifc.py <rvt-rs.ifc> <revit.ifc> [--list]
"""

import argparse
import math

import ifcopenshell
import ifcopenshell.util.placement as placement
import ifcopenshell.util.unit as unit
import numpy

FT = 0.3048
END_TOLERANCE_FT = 0.01
RADIUS_TOLERANCE_FT = 0.001
LENGTH_TOLERANCE_FT = 0.01


def profile_radius(profile):
    """A circular profile's radius in the file's units: a circle's, or that
    of a closed polyline, or of an indexed curve of arcs only (Revit writes
    the BIM4LCA office's pipes as four arcs through eight points), whose
    points all lie within 0.1% of one distance from their centroid; None for
    any other profile."""
    if profile.is_a("IfcCircleProfileDef"):
        return float(profile.Radius)
    curve = getattr(profile, "OuterCurve", None)
    if curve is None:
        return None
    if curve.is_a("IfcPolyline"):
        points = [tuple(float(c) for c in p.Coordinates[:2]) for p in curve.Points]
    elif curve.is_a("IfcIndexedPolyCurve") and curve.Segments and all(seg.is_a("IfcArcIndex") for seg in curve.Segments):
        # Arcs through points of the list: every point is on the circle.
        points = [tuple(float(c) for c in p[:2]) for p in curve.Points.CoordList]
    else:
        return None
    if len(points) > 1 and points[0] == points[-1]:
        points = points[:-1]
    if len(points) < 8:
        return None
    cx = sum(p[0] for p in points) / len(points)
    cy = sum(p[1] for p in points) / len(points)
    distances = [math.dist(p, (cx, cy)) for p in points]
    mean = sum(distances) / len(distances)
    if mean <= 0 or max(abs(d - mean) for d in distances) > 1e-3 * mean:
        return None
    return mean


def describe(profile):
    """What a profile is, for one that profile_radius does not read."""
    curve = getattr(profile, "OuterCurve", None)
    if curve is None:
        return profile.is_a()
    text = f"{profile.is_a()} {curve.is_a()}"
    if curve.is_a("IfcPolyline"):
        points = [tuple(float(c) for c in p.Coordinates[:2]) for p in curve.Points]
        cx = sum(p[0] for p in points) / len(points)
        cy = sum(p[1] for p in points) / len(points)
        d = [math.dist(p, (cx, cy)) for p in points]
        text += f" {len(points)} points, from their centroid {min(d):.5f} to {max(d):.5f}"
    elif curve.is_a("IfcIndexedPolyCurve"):
        text += f" {len(curve.Points.CoordList)} points, segments {[s.is_a() for s in (curve.Segments or [])][:4]}"
    elif curve.is_a("IfcCompositeCurve"):
        text += f" {[seg.ParentCurve.is_a() for seg in curve.Segments][:6]}"
    return text


def cylinders(path):
    """Tag -> (profile name, radius in feet or None, start, end, length in feet, Name, profile)."""
    model = ifcopenshell.open(path)
    scale = unit.calculate_unit_scale(model) / FT
    # Each file in its own site frame: Revit writes the BIM4LCA office's
    # site at its shared coordinates, rvt-rs at its internal ones.
    sites = model.by_type("IfcSite")
    inverse = numpy.linalg.inv(placement.get_local_placement(sites[0].ObjectPlacement)) if sites else numpy.eye(4)
    out = {}
    for element in model.by_type("IfcFlowSegment"):
        if not element.Tag or not element.Representation:
            continue
        world = inverse @ placement.get_local_placement(element.ObjectPlacement)
        for representation in element.Representation.Representations:
            for item in representation.Items:
                if not item.is_a("IfcExtrudedAreaSolid"):
                    continue
                matrix = world @ placement.get_axis2placement(item.Position)
                start = [float(v) * scale for v in matrix[:3, 3]]
                ratios = [float(v) for v in item.ExtrudedDirection.DirectionRatios]
                norm = math.sqrt(sum(v * v for v in ratios))
                local = [v / norm for v in ratios]
                direction = [sum(float(matrix[i][j]) * local[j] for j in range(3)) for i in range(3)]
                length = float(item.Depth) * scale
                end = [start[i] + direction[i] * length for i in range(3)]
                profile = item.SweptArea
                radius = profile_radius(profile)
                if radius is not None:
                    radius *= scale
                out[element.Tag] = (profile.is_a(), radius, start, end, length, element.Name or "", profile)
    return out


def end_error(a, b):
    """Largest end distance of two axes, ends taken in the closer order."""
    straight = max(math.dist(a[2], b[2]), math.dist(a[3], b[3]))
    swapped = max(math.dist(a[2], b[3]), math.dist(a[3], b[2]))
    return min(straight, swapped)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("ours")
    ap.add_argument("reference")
    ap.add_argument("--list", action="store_true", help="print every pipe that differs")
    args = ap.parse_args()
    ours = cylinders(args.ours)
    # Revit's IFC2X3 export writes ducts as IfcFlowSegment too; its pipes are
    # named "Pipe Types:<type>:<ElementId>".
    ref = {t: v for t, v in cylinders(args.reference).items() if v[5].startswith("Pipe Types:")}
    both = sorted(set(ours) & set(ref), key=lambda t: (len(t), t))
    print(f"pipes in Revit's export with an extruded body: {len(ref)}; rvt-rs writes one for {len(both)} of them")
    profiles = {}
    for tag in both:
        profiles[ours[tag][0]] = profiles.get(ours[tag][0], 0) + 1
    print("rvt-rs profiles on those:", dict(sorted(profiles.items())))
    theirs = {}
    for tag in both:
        key = f"{ref[tag][0]}{'' if ref[tag][1] is not None else ' (not circular)'}"
        theirs[key] = theirs.get(key, 0) + 1
    print("Revit's profiles on those:", dict(sorted(theirs.items())))
    other = next((ref[tag][6] for tag in both if ref[tag][1] is None), None)
    if other is not None:
        print("  one that is not read as a circle:", describe(other))
    if not both:
        return
    circle = radius_ok = ends_ok = length_ok = all_ok = 0
    worst_radius = worst_end = 0.0
    worst_radii = None
    misses = []
    for tag in both:
        a, b = ours[tag], ref[tag]
        is_circle = a[0] == "IfcCircleProfileDef"
        r_err = abs(a[1] - b[1]) if is_circle and b[1] is not None else None
        e_err = end_error(a, b)
        l_err = abs(a[4] - b[4])
        circle += is_circle
        radius_ok += r_err is not None and r_err <= RADIUS_TOLERANCE_FT
        ends_ok += e_err <= END_TOLERANCE_FT
        length_ok += l_err <= LENGTH_TOLERANCE_FT
        good = (
            r_err is not None
            and r_err <= RADIUS_TOLERANCE_FT
            and e_err <= END_TOLERANCE_FT
            and l_err <= LENGTH_TOLERANCE_FT
        )
        all_ok += good
        if r_err is not None and r_err > worst_radius:
            worst_radius, worst_radii = r_err, (tag, a[1], b[1])
        worst_end = max(worst_end, e_err)
        if not good:
            misses.append((tag, a[0], r_err, e_err, l_err))
    n = len(both)
    print(f"circle profile: {circle} of {n}")
    print(f"radius within {RADIUS_TOLERANCE_FT} ft of Revit's: {radius_ok} of {n} (worst {worst_radius:.5f} ft)")
    if worst_radii is not None and worst_radius > RADIUS_TOLERANCE_FT:
        tag, mine, theirs = worst_radii
        print(f"  worst radius: {tag}, rvt-rs {mine:.5f} ft, Revit {theirs:.5f} ft")
    print(f"axis ends within {END_TOLERANCE_FT} ft of Revit's: {ends_ok} of {n} (worst {worst_end:.5f} ft)")
    print(f"length within {LENGTH_TOLERANCE_FT} ft of Revit's: {length_ok} of {n}")
    print(f"all four: {all_ok} of {n}")
    if args.list:
        for tag, profile, r_err, e_err, l_err in misses:
            r_text = "-" if r_err is None else f"{r_err:.5f}"
            print(f"  {tag}: {profile} radius err {r_text}, ends err {e_err:.5f}, length err {l_err:.5f}")


if __name__ == "__main__":
    main()

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

FT = 0.3048
END_TOLERANCE_FT = 0.01
RADIUS_TOLERANCE_FT = 0.001
LENGTH_TOLERANCE_FT = 0.01


def cylinders(path):
    """Tag -> (profile name, radius in feet or None, start, end, length in feet)."""
    model = ifcopenshell.open(path)
    scale = unit.calculate_unit_scale(model) / FT
    out = {}
    for element in model.by_type("IfcFlowSegment"):
        if not element.Tag or not element.Representation:
            continue
        world = placement.get_local_placement(element.ObjectPlacement)
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
                radius = None
                if profile.is_a("IfcCircleProfileDef"):
                    radius = float(profile.Radius) * scale
                out[element.Tag] = (profile.is_a(), radius, start, end, length)
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
    ref = cylinders(args.reference)
    both = sorted(set(ours) & set(ref), key=lambda t: (len(t), t))
    print(f"pipes with an extruded body: Revit {len(ref)}, rvt-rs {len(ours)}, both {len(both)}")
    profiles = {}
    for tag in both:
        profiles[ours[tag][0]] = profiles.get(ours[tag][0], 0) + 1
    print("rvt-rs profiles on those:", dict(sorted(profiles.items())))
    if not both:
        return
    circle = radius_ok = ends_ok = length_ok = all_ok = 0
    worst_radius = worst_end = 0.0
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
        if r_err is not None:
            worst_radius = max(worst_radius, r_err)
        worst_end = max(worst_end, e_err)
        if not good:
            misses.append((tag, a[0], r_err, e_err, l_err))
    n = len(both)
    print(f"circle profile: {circle} of {n}")
    print(f"radius within {RADIUS_TOLERANCE_FT} ft of Revit's: {radius_ok} of {n} (worst {worst_radius:.5f} ft)")
    print(f"axis ends within {END_TOLERANCE_FT} ft of Revit's: {ends_ok} of {n} (worst {worst_end:.5f} ft)")
    print(f"length within {LENGTH_TOLERANCE_FT} ft of Revit's: {length_ok} of {n}")
    print(f"all four: {all_ok} of {n}")
    if args.list:
        for tag, profile, r_err, e_err, l_err in misses:
            r_text = "-" if r_err is None else f"{r_err:.5f}"
            print(f"  {tag}: {profile} radius err {r_text}, ends err {e_err:.5f}, length err {l_err:.5f}")


if __name__ == "__main__":
    main()

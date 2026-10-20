#!/usr/bin/env python3
"""Where Revit's export places a pipe's ports, against the ends of rvt-rs's
pipe (#847).

Revit writes each connector as an `IfcDistributionPort` named
`<In or Out>Port_<ElementId>_<index>`, placed at the connector. rvt-rs draws
a pipe as a cylinder from the first end its connector entries give to the
second (RE-131). For every port of Revit's export whose ElementId is a pipe
rvt-rs draws, this says whether the port lies at rvt-rs's first end, its
second end or neither, by connector index, and how the port's Z axis points
against the pipe's axis there (outward, inward, or across). A rule "connector
`i` is end `i`" holds only if every index-0 port is at the first end and
every index-1 port at the second.

Needs IfcOpenShell (`pip install ifcopenshell`).

Usage:

    python3 tools/re/port_places_vs_ifc.py <rvt-rs.ifc> <revit.ifc> [--list]
"""

import argparse
import collections
import math
import re

import ifcopenshell
import ifcopenshell.util.placement as placement
import ifcopenshell.util.unit as unit

FT = 0.3048
TOLERANCE_FT = 0.01
PORT_NAME = re.compile(r"^(?:In|Out)?Port_(\d+)_(\d+)$")


def pipe_axes(path):
    """Tag -> (first end, second end), feet, of each extruded flow segment."""
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
                depth = float(item.Depth) * scale
                out[element.Tag] = (start, [start[i] + direction[i] * depth for i in range(3)])
    return out


def ports(path):
    """(ElementId, index, In/Out) -> (point in feet, Z axis) of each named port."""
    model = ifcopenshell.open(path)
    scale = unit.calculate_unit_scale(model) / FT
    out = {}
    for port in model.by_type("IfcDistributionPort"):
        match = PORT_NAME.match(port.Name or "")
        if not match or not port.ObjectPlacement:
            continue
        matrix = placement.get_local_placement(port.ObjectPlacement)
        point = [float(v) * scale for v in matrix[:3, 3]]
        axis = [float(matrix[i][2]) for i in range(3)]
        side = "In" if (port.Name or "").startswith("In") else "Out" if (port.Name or "").startswith("Out") else "-"
        out[(match.group(1), int(match.group(2)))] = (point, axis, side)
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("ours")
    ap.add_argument("reference")
    ap.add_argument("--list", action="store_true")
    args = ap.parse_args()
    axes = pipe_axes(args.ours)
    theirs = ports(args.reference)
    counts = collections.Counter()
    directions = collections.Counter()
    sides = collections.Counter()
    listed = []
    for (tag, index), (point, axis, side) in sorted(theirs.items()):
        if tag not in axes:
            continue
        first, second = axes[tag]
        d_first, d_second = math.dist(point, first), math.dist(point, second)
        if d_first <= TOLERANCE_FT:
            at, end, other = "first end", first, second
        elif d_second <= TOLERANCE_FT:
            at, end, other = "second end", second, first
        else:
            at, end, other = "neither end", None, None
        counts[(index, at)] += 1
        sides[(index, side)] += 1
        if end is not None:
            outward = [end[i] - other[i] for i in range(3)]
            norm = math.sqrt(sum(v * v for v in outward)) or 1.0
            cos = sum(axis[i] * outward[i] / norm for i in range(3))
            directions["outward" if cos > 0.99 else "inward" if cos < -0.99 else "across"] += 1
        if args.list or at == "neither end":
            listed.append(f"  {tag}_{index}: {at}, {min(d_first, d_second):.4f} ft from the nearer end")
    pipes = len({tag for tag, _ in theirs if tag in axes})
    print(f"Revit's ports on pipes rvt-rs draws as a cylinder: {sum(counts.values())} on {pipes} pipes")
    for (index, at), n in sorted(counts.items()):
        print(f"  index {index}: {at} {n}")
    print("  port Z axis against the pipe at that end: " + ", ".join(f"{k} {v}" for k, v in directions.most_common()))
    print("  In/Out by index: " + ", ".join(f"{i} {s} {n}" for (i, s), n in sorted(sides.items())))
    for line in listed[:60]:
        print(line)


if __name__ == "__main__":
    main()

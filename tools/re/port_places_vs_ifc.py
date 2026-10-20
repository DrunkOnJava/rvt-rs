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
    mine = ports(args.ours)
    counts = collections.Counter()
    directions = collections.Counter()
    by_side = collections.Counter()
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
            direction = "outward" if cos > 0.99 else "inward" if cos < -0.99 else "across"
            directions[direction] += 1
            by_side[(side, direction)] += 1
        if args.list or at == "neither end":
            listed.append(f"  {tag}_{index}: {at}, {min(d_first, d_second):.4f} ft from the nearer end")
    pipes = len({tag for tag, _ in theirs if tag in axes})
    print(f"Revit's ports on pipes rvt-rs draws as a cylinder: {sum(counts.values())} on {pipes} pipes")
    for (index, at), n in sorted(counts.items()):
        print(f"  index {index}: {at} {n}")
    print("  port Z axis against the pipe at that end: " + ", ".join(f"{k} {v}" for k, v in directions.most_common()))
    print("  In/Out by index: " + ", ".join(f"{i} {s} {n}" for (i, s), n in sorted(sides.items())))
    print("  Z axis by In/Out: " + ", ".join(f"{s} {d} {n}" for (s, d), n in sorted(by_side.items())))
    # #847: rvt-rs's own placement of the same ports.
    placed = collections.Counter()
    for (tag, index), (point, axis, side) in sorted(theirs.items()):
        if tag not in axes:
            continue
        own = mine.get((tag, index))
        if own is None:
            placed["not placed by rvt-rs"] += 1
        elif math.dist(own[0], point) <= TOLERANCE_FT:
            placed[f"placed within {TOLERANCE_FT} ft of Revit's"] += 1
            cos = sum(own[1][i] * axis[i] for i in range(3))
            placed[f"Z axis {'as' if cos > 0.99 else 'opposite to' if cos < -0.99 else 'across'} Revit's ({side})"] += 1
        else:
            placed["placed elsewhere"] += 1
            listed.append(f"  {tag}_{index}: rvt-rs's port {math.dist(own[0], point):.4f} ft from Revit's")
    print("rvt-rs's ports on those pipes: " + ", ".join(f"{k} {v}" for k, v in placed.most_common()))
    # #869: where Revit's export places a port joined to a flow segment's
    # port, against that segment port.
    model = ifcopenshell.open(args.reference)
    scale = unit.calculate_unit_scale(model) / FT

    def place(port):
        matrix = placement.get_local_placement(port.ObjectPlacement)
        return [float(v) * scale for v in matrix[:3, 3]], [float(matrix[i][2]) for i in range(3)]

    def owner(port):
        for rel in getattr(port, "ContainedIn", None) or []:
            return rel.RelatedElement
        for rel in getattr(port, "Nests", None) or []:
            return rel.RelatingObject
        return None

    joins = collections.Counter()
    for rel in model.by_type("IfcRelConnectsPorts"):
        pair = [rel.RelatingPort, rel.RelatedPort]
        owners = [owner(p) for p in pair]
        kinds = ["segment" if o is not None and o.is_a("IfcFlowSegment") else "other" for o in owners]
        if kinds.count("segment") != 1 or not all(p.ObjectPlacement for p in pair):
            continue
        seg, other = (pair[0], pair[1]) if kinds[0] == "segment" else (pair[1], pair[0])
        (sp, sz), (op, oz) = place(seg), place(other)
        cos = sum(sz[i] * oz[i] for i in range(3))
        joins[
            (
                "at the segment port" if math.dist(sp, op) <= TOLERANCE_FT else f"{math.dist(sp, op):.3f} ft away",
                "opposite axis" if cos < -0.99 else "same axis" if cos > 0.99 else "other axis",
            )
        ] += 1
    print(
        "Revit's ports joined to a segment's port: "
        + ", ".join(f"{a}, {b} {n}" for (a, b), n in joins.most_common(8))
    )
    for line in listed[:60]:
        print(line)


if __name__ == "__main__":
    main()

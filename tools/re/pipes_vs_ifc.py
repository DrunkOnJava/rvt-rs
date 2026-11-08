#!/usr/bin/env python3
"""Each pipe's own record against Revit's export of it (RE-370).

Reads `probe_re370_pipes --native`'s lines (`ElementId start x,y,z end x,y,z
outer <ft>`, model feet: a pipe's one `GLine` and its `m_dOuterDiameter`) and
Revit's IFC, and says for each pipe of the export that the probe gives, by
ElementId (`Tag`), whether the record's line gives Revit's `Length`, the
record's cylinder Revit's `OuterSurfaceArea` and `GrossCrossSectionArea`
(where written), and a vertical pipe's lower end Revit's `InvertElevation`.
A pipe is an `IfcPipeSegment` in IFC4 and an `IfcFlowSegment` in IFC2X3.

Needs IfcOpenShell (`pip install ifcopenshell`).

Usage:

    python3 tools/re/pipes_vs_ifc.py <probe output> <revit.ifc>
"""

import argparse
import collections
import math

import ifcopenshell
import ifcopenshell.util.element as element
import ifcopenshell.util.unit as unit

FT = 0.3048
LENGTH_M = 1e-4
AREA_RELATIVE = 1e-3
AXIS_FT = 1e-4


def read_probe(path):
    pipes = {}
    with open(path, encoding="utf-8") as lines:
        for line in lines:
            words = line.split()
            if len(words) != 7 or words[1] != "start":
                continue
            start = [float(v) for v in words[2].split(",")]
            end = [float(v) for v in words[4].split(",")]
            pipes[words[0]] = (start, end, float(words[6]))
    return pipes


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("probe")
    parser.add_argument("ifc")
    args = parser.parse_args()
    pipes = read_probe(args.probe)
    model = ifcopenshell.open(args.ifc)
    scale = unit.calculate_unit_scale(model)
    entity = "IfcFlowSegment" if model.schema == "IFC2X3" else "IfcPipeSegment"
    scores = collections.defaultdict(lambda: [0, 0])
    misses = []
    compared = 0
    for segment in model.by_type(entity):
        if segment.Tag not in pipes:
            continue
        compared += 1
        start, end, outer = pipes[segment.Tag]
        length = math.dist(start, end) * FT
        radius = outer / 2 * FT
        values = {}
        for properties in element.get_psets(segment).values():
            values.update(properties)

        def score(name, ok):
            scores[name][0] += ok
            scores[name][1] += 1
            if not ok:
                misses.append((segment.Tag, name))

        if "Length" in values:
            score("Length", abs(values["Length"] * scale - length) <= LENGTH_M)
        if "OuterSurfaceArea" in values:
            want = 2 * math.pi * radius * length
            score("OuterSurfaceArea", abs(values["OuterSurfaceArea"] - want) <= AREA_RELATIVE * want)
        if "GrossCrossSectionArea" in values:
            want = math.pi * radius * radius
            score(
                "GrossCrossSectionArea",
                abs(values["GrossCrossSectionArea"] - want) <= AREA_RELATIVE * want,
            )
        vertical = (
            abs(start[0] - end[0]) <= AXIS_FT
            and abs(start[1] - end[1]) <= AXIS_FT
            and abs(start[2] - end[2]) > AXIS_FT
        )
        if "InvertElevation" in values and vertical:
            want = min(start[2], end[2]) * FT
            score("InvertElevation (vertical)", abs(values["InvertElevation"] * scale - want) <= LENGTH_M)
    print(f"{args.ifc}: {compared} of the export's pipes are in the probe output")
    for name, (ok, total) in sorted(scores.items()):
        print(f"  {name}: {ok} of {total} as Revit's")
    for tag, name in misses[:20]:
        print(f"  miss {tag} {name}")


if __name__ == "__main__":
    main()

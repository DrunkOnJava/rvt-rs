#!/usr/bin/env python3
"""What Revit's IFC4 export writes on the openings of a wall's edited elevation
profile (RE-380, #1607): each opening whose Tag is the ElementId of one of the
host wall's sketch lines, with the host's own `Reference`, the opening's
property sets and quantity sets, and the area of its profile.

The probe `probe_1607_wall_profile_lines` prints each wall's sketch lines and
their ElementIds (`wall <id> box ... lines <n>`, then `  line <id> (...) (...)`).
This reads that output and Revit's export, and prints, for each wall with such
an opening, the wall's type name and `Pset_WallCommon.Reference`, then each
opening's Tag, Name, every property set and quantity set it carries (values as
written) and the area of its profile in square metres, so that a rule for the
values can be read off several openings at once. It scores nothing.

Usage:

    python3 tools/re/pocket_opening_values_vs_ifc.py <probe output> <revit-export.ifc>

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import re
import sys

import ifcopenshell
import ifcopenshell.util.element
import ifcopenshell.util.unit


def sketch_lines(path):
    """Wall ElementId (as text) -> the ElementIds (as text) of its sketch lines."""
    walls = {}
    current = None
    with open(path, errors="replace") as lines:
        for line in lines:
            wall = re.match(r"wall (\d+) box ", line)
            if wall:
                current = walls.setdefault(wall.group(1), set())
                continue
            sketch = re.match(r"\s+line (\d+) ", line)
            if sketch and current is not None:
                current.add(sketch.group(1))
    return walls


def solids(item):
    if item.is_a("IfcBooleanResult"):
        yield from solids(item.FirstOperand)
        yield from solids(item.SecondOperand)
    elif item.is_a("IfcExtrudedAreaSolid"):
        yield item


def profile_area_square_metres(opening, scale):
    """The area of the opening's first extruded profile, or None."""
    if opening.Representation is None:
        return None
    for representation in opening.Representation.Representations:
        for item in representation.Items:
            for solid in solids(item):
                curve = getattr(solid.SweptArea, "OuterCurve", None)
                if curve is None:
                    continue
                if curve.is_a("IfcIndexedPolyCurve"):
                    points = [tuple(p[:2]) for p in curve.Points.CoordList]
                else:
                    points = [tuple(p.Coordinates[:2]) for p in curve.Points]
                if len(points) > 1 and points[0] == points[-1]:
                    points = points[:-1]
                twice = sum(
                    points[i][0] * points[(i + 1) % len(points)][1]
                    - points[(i + 1) % len(points)][0] * points[i][1]
                    for i in range(len(points))
                )
                return abs(twice) / 2 * scale * scale
    return None


def main():
    probe, export = sys.argv[1], sys.argv[2]
    walls = sketch_lines(probe)
    f = ifcopenshell.open(export)
    scale = ifcopenshell.util.unit.calculate_unit_scale(f)
    found = 0
    for rel in f.by_type("IfcRelVoidsElement"):
        host = rel.RelatingBuildingElement
        opening = rel.RelatedOpeningElement
        if not host.is_a("IfcWall") or opening.Tag not in walls.get(host.Tag, ()):
            continue
        found += 1
        host_sets = ifcopenshell.util.element.get_psets(host)
        host_type = ifcopenshell.util.element.get_type(host)
        print(
            f"wall {host.Tag}: type name {host_type.Name if host_type else None!r}, "
            f"Pset_WallCommon.Reference {host_sets.get('Pset_WallCommon', {}).get('Reference')!r}"
        )
        area = profile_area_square_metres(opening, scale)
        print(
            f"  opening {opening.Tag}: name {opening.Name!r}, predefined {opening.PredefinedType}, "
            f"profile area {area:.6f} m2" if area is not None else
            f"  opening {opening.Tag}: name {opening.Name!r}, predefined {opening.PredefinedType}, no profile"
        )
        for name, values in sorted(ifcopenshell.util.element.get_psets(opening).items()):
            written = {key: value for key, value in sorted(values.items()) if key != "id"}
            print(f"    {name}: {written}")
    print(f"pocket openings in Revit's export: {found}")


if __name__ == "__main__":
    main()

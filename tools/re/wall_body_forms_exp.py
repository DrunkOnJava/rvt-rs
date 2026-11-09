#!/usr/bin/env python3
"""Throwaway (#1607): how each IFC file writes the body of the listed walls and
which openings void them.

Usage: python3 tools/re/wall_body_forms_exp.py <rvt-rs.ifc> <revit-export.ifc>
"""

import sys

import ifcopenshell

WALLS = (
    "34362", "55840", "1351524", "1387731", "2056705", "2056770", "2057668",
    "2057670", "2058499", "2058501", "2070689", "2070691", "3025796", "3026297",
    "3027380",
)


def describe(item):
    kind = item.is_a()
    if kind == "IfcExtrudedAreaSolid":
        direction = tuple(round(x, 3) for x in item.ExtrudedDirection.DirectionRatios)
        return f"{kind}(profile {item.SweptArea.is_a()}, direction {direction}, depth {item.Depth:.4f})"
    if kind in ("IfcBooleanClippingResult", "IfcBooleanResult"):
        return f"{kind}({item.Operator}: {describe(item.FirstOperand)} ; {describe(item.SecondOperand)})"
    return kind


def walls(path):
    model = ifcopenshell.open(path)
    out = {}
    for wall in model.by_type("IfcWall"):
        if wall.Tag not in WALLS:
            continue
        body = None
        if wall.Representation:
            for representation in wall.Representation.Representations:
                if representation.RepresentationIdentifier == "Body":
                    body = (representation.RepresentationType, [describe(i) for i in representation.Items])
        voids = [(rel.RelatedOpeningElement.Tag, rel.RelatedOpeningElement.is_a()) for rel in (wall.HasOpenings or [])]
        out.setdefault(wall.Tag, []).append((wall.is_a(), body, voids))
    return out


def main(argv):
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    ours, revit = walls(argv[1]), walls(argv[2])
    for tag in WALLS:
        print(f"wall {tag}:")
        for label, side in (("revit", revit), ("rvt-rs", ours)):
            rows = side.get(tag)
            if not rows:
                print(f"  {label}: no IfcWall with this Tag")
                continue
            for entity, body, voids in rows:
                print(f"  {label}: {entity}; Body {body}; openings {voids}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

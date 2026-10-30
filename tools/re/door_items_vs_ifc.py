#!/usr/bin/env python3
"""Revit's body items for the four BIM4LCA office doors of #1103, one by one.

Research tool for RE-265. For each door, by Tag, it tessellates every item of
the door's Body representation (through its mapped item) with IfcOpenShell
and prints the item's entity, its triangle count, its signed volume and its
bounding box, in cubic feet and feet, then the sum. rvt-rs's saved-graphics
objects for the same doors are the probe's output (the first argument),
copied through.

Usage: door_items_vs_ifc.py <probe output> <revit-export.ifc>
"""

import sys

import ifcopenshell
import ifcopenshell.geom
import numpy as np

DOORS = ("1382481", "1623922", "3448563", "3444654")
FEET = 0.3048


def items(representation):
    for item in representation.Items:
        if item.is_a("IfcMappedItem"):
            yield from items(item.MappingSource.MappedRepresentation)
        else:
            yield item


def main(argv):
    argv = [a for a in argv if not a.startswith("--")]
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    for line in open(argv[1], encoding="utf-8"):
        if line.startswith(("door ", "  object ")):
            print("rvt-rs " + line.rstrip())
    f = ifcopenshell.open(argv[2])
    settings = ifcopenshell.geom.settings()
    for door in f.by_type("IfcDoor"):
        if door.Tag not in DOORS:
            continue
        total = 0.0
        for rep in door.Representation.Representations:
            if rep.RepresentationIdentifier != "Body":
                continue
            for item in items(rep):
                try:
                    shape = ifcopenshell.geom.create_shape(settings, item)
                except Exception as error:
                    print(f"Revit {door.Tag} {item.is_a()} #{item.id()}: not tessellated ({error})")
                    continue
                v = np.array(shape.verts, dtype=float).reshape(-1, 3) / FEET
                t = np.array(shape.faces, dtype=int).reshape(-1, 3)
                a, b, c = v[t[:, 0]], v[t[:, 1]], v[t[:, 2]]
                signed = float(np.einsum("ij,ij->i", a, np.cross(b, c)).sum() / 6.0)
                total += signed
                size = v.max(axis=0) - v.min(axis=0)
                print(
                    f"Revit {door.Tag} {item.is_a()} #{item.id()}: {len(t)} triangles, signed volume"
                    f" {signed:.4f} cubic ft, box {size[0]:.4f} x {size[1]:.4f} x {size[2]:.4f} ft"
                )
        print(f"Revit {door.Tag}: sum of signed volumes {total:.4f} cubic ft")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

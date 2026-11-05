#!/usr/bin/env python3
"""Score rvt-rs floors and footings drawn along their shape-edited tops against
the body Revit's own IFC export writes for the same element (#1384, RE-336).

A floor whose corner vertices are shape-edited (`SlabShapeEditGStep`) has
`BodySource` `native_slab_shape_edit` in the rvt-rs export: the slab between its
sloped top and the same surface one thickness lower. For every such element
that Revit's export also holds (same `Tag`) this prints what
`mesh_bodies_vs_ifc.py` prints for the saved meshes: per class and under it per
Revit type, how many have Revit's volume within 1% and 0.01%, every vertex of
each body within 0.01 ft and 0.001 ft of the other's surface, and "ref on
ours", then the elements furthest off. It is that scorer with another source.

Before that it counts the trust markers the same change writes, by class and
value: `SlabShapeRefusal` (why a floor whose corners are shape-edited is still
its record box), `BaseCoverRefusal` (why a wall whose base join names such a
floor keeps its base), `BodyBaseSource` (how a wall's base was cut) and
`BodyHeightSource` (how its top was). A wall's base against Revit's is scored by `wall_heights_vs_ifc.py`.

Needs IfcOpenShell and numpy.

Usage:

    python3 tools/re/slab_bodies_vs_ifc.py <rvt-rs.ifc> <revit.ifc>
"""

import collections
import os
import sys

import ifcopenshell

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mesh_bodies_vs_ifc  # noqa: E402

SOURCE = "native_slab_shape_edit"
MARKERS = ("SlabShapeRefusal", "BaseCoverRefusal", "BodyBaseSource", "BodyHeightSource")


def marker_counts(path):
    counts = collections.defaultdict(collections.Counter)
    model = ifcopenshell.open(path)
    for element in model.by_type("IfcElement"):
        if element.is_a("IfcOpeningElement"):
            continue
        for rel in element.IsDefinedBy:
            if not rel.is_a("IfcRelDefinesByProperties"):
                continue
            for prop in getattr(rel.RelatingPropertyDefinition, "HasProperties", None) or []:
                if prop.Name in MARKERS and prop.NominalValue is not None:
                    counts[prop.Name][(element.is_a(), prop.NominalValue.wrappedValue)] += 1
    return counts


def main():
    if len(sys.argv) != 3:
        print(__doc__)
        return 2
    counts = marker_counts(sys.argv[1])
    for name in MARKERS:
        if not counts[name]:
            print(f"{name}: none")
        for (cls, value), n in sorted(counts[name].items()):
            print(f"{name} {cls} {value}: {n}")
    mesh_bodies_vs_ifc.SOURCE = SOURCE
    return mesh_bodies_vs_ifc.main()


if __name__ == "__main__":
    sys.exit(main())

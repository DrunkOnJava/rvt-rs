#!/usr/bin/env python3
"""Score the shape of rvt-rs's IfcShadingDevice bodies against the body Revit's
own IFC export writes for the same element (#1492, RE-78).

Core Interior's 20 shading devices are floors that Revit's export writes as
`IfcShadingDevice`. They were record boxes until RE-95 closed their sketch
outlines, so they are drawn from those outlines, and `saved_meshes::attach`
leaves a body whose profile is resolved alone: they are never saved-mesh
bodies, and `mesh_bodies_vs_ifc.py`, which scores `BodySource`
`saved_graphics_mesh`, does not meet them. `plan_profiles_vs_ifc.py` scores
their plan outline and not their thickness. This is `mesh_bodies_vs_ifc.py`
with another selection, every `IfcShadingDevice` with a Tag whatever its
`BodySource`: per class and under it per Revit type, how many have Revit's
volume within 1% and 0.01%, every vertex of each body within 0.01 ft and
0.001 ft of the other's surface, and "ref on ours", then the elements
furthest off.

Before that it counts what drew the bodies it scores, by class: their
`BodySource` and `ProfileResolved`, so the table says whether the plates are
a saved mesh, a record box or an outline.

Needs IfcOpenShell and numpy.

Usage:

    python3 tools/re/shading_bodies_vs_ifc.py <rvt-rs.ifc> <revit.ifc>
"""

import collections
import os
import sys

import ifcopenshell

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mesh_bodies_vs_ifc  # noqa: E402

CLASS = "IfcShadingDevice"
SOURCE = "shading_device"
MARKERS = ("BodySource", "ProfileResolved")


def markers(element):
    found = {}
    for rel in element.IsDefinedBy:
        if not rel.is_a("IfcRelDefinesByProperties"):
            continue
        for prop in getattr(rel.RelatingPropertyDefinition, "HasProperties", None) or []:
            if prop.Name in MARKERS and prop.NominalValue is not None:
                found[prop.Name] = prop.NominalValue.wrappedValue
    return found


def main():
    if len(sys.argv) != 3:
        print(__doc__)
        return 2
    counts = collections.Counter()
    model = ifcopenshell.open(sys.argv[1])
    for element in model.by_type(CLASS):
        if element.Tag:
            found = markers(element)
            counts[tuple(found.get(name) for name in MARKERS)] += 1
    del model
    if not counts:
        print(f"{CLASS} with a Tag: none")
    for (source, resolved), n in sorted(counts.items(), key=lambda item: (-item[1], str(item[0]))):
        print(f"{CLASS} BodySource {source} ProfileResolved {resolved}: {n}")
    mesh_bodies_vs_ifc.SOURCE = SOURCE
    mesh_bodies_vs_ifc.body_sources = lambda element: SOURCE if element.is_a(CLASS) else None
    return mesh_bodies_vs_ifc.main()


if __name__ == "__main__":
    sys.exit(main())

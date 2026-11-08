#!/usr/bin/env python3
"""List each IfcPlate's type and materials in rvt-rs's export against Revit's (#1520).

A research lister for an openly licensed model: it prints ElementIds and names.

Usage:

    python3 tools/re/plate_types_vs_ifc.py <model.ifc> <revit-export.ifc>
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.util.element as ue


def materials(entity):
    found = ue.get_material(entity, should_skip_usage=True)
    if found is None:
        return ()
    names = []
    for attr in ("MaterialLayers", "MaterialConstituents", "Materials", "MaterialProfiles"):
        for item in getattr(found, attr, None) or []:
            mat = getattr(item, "Material", item)
            names.append(getattr(mat, "Name", None))
    if not names:
        names.append(getattr(found, "Name", None))
    return tuple(sorted(str(n) for n in names))


def rows(path):
    out = {}
    # Entities read from a file whose object has been collected crash the process.
    model = ifcopenshell.open(path)
    for plate in model.by_type("IfcPlate"):
        kind = ue.get_type(plate)
        out[plate.Tag] = (
            kind.Tag if kind else None,
            kind.Name if kind else None,
            plate.ObjectType,
            materials(plate),
            materials(kind) if kind else (),
        )
    return out


ours, theirs = rows(sys.argv[1]), rows(sys.argv[2])
both = sorted(set(ours) & set(theirs))
print(f"plates: rvt-rs {len(ours)}, Revit {len(theirs)}, both {len(both)}")
shape = collections.Counter()
for tag in both:
    a, b = ours[tag], theirs[tag]
    shape[(a[0] == b[0], a[1] == b[1], a[3] == b[3])] += 1
print("(type Tag equal, type Name equal, material set equal): count")
for key, count in sorted(shape.items(), key=lambda x: -x[1]):
    print(f"  {key}: {count}")
pairs = collections.Counter()
for tag in both:
    a, b = ours[tag], theirs[tag]
    pairs[(a[0], b[0], a[1], b[1], a[3], b[3])] += 1
print("distinct (our type Tag, Revit's type Tag, our Name, Revit's Name, our materials, Revit's materials): count")
for key, count in sorted(pairs.items(), key=lambda x: -x[1])[:40]:
    print(f"  {count} {key}")

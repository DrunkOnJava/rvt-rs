#!/usr/bin/env python3
"""Compare walls' material associations in `rvt-ifc` exports with Revit's.

Research tool for `reports/element-framing/RE-88-wall-materials-without-layer-sets.md`.
It is an end-to-end check against Revit's own IFC4 export, read with
IfcOpenShell.

For each wall in both files it classes rvt-rs's association against
Revit's: a layer set (RE-58, reported by Revit's kind), none, or the same
kind as Revit's with the same materials in the same order and, for a
constituent set, the same constituent names.

It also checks Revit's naming rule on the walls rvt-rs writes a layer set
for: one layer gives an `IfcMaterial` of that name; several give a
constituent set whose constituents are the layer materials in order, the
k-th occurrence of a material named " (k)" after it.

Usage:

    python3 tools/re/wall_materials_vs_ifc.py <revit-export.ifc> <rvt-rs.ifc>...

Needs IfcOpenShell (tested with 0.8.5).
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.util.element


def name(material):
    return material.Name if material else None


def association(material):
    if material is None:
        return ("none",)
    if material.is_a("IfcMaterialLayerSetUsage"):
        return ("layer set", tuple(name(layer.Material) for layer in material.ForLayerSet.MaterialLayers))
    if material.is_a("IfcMaterialConstituentSet"):
        return (
            "constituent set",
            tuple(name(c.Material) for c in material.MaterialConstituents),
            tuple(c.Name for c in material.MaterialConstituents),
        )
    if material.is_a("IfcMaterial"):
        return ("material", (material.Name,))
    return (material.is_a(),)


def numbered(names):
    seen = collections.Counter()
    out = []
    for n in names:
        seen[n] += 1
        out.append(n if seen[n] == 1 else f"{n} ({seen[n]})")
    return tuple(out)


def main(argv):
    if len(argv) < 3:
        raise SystemExit(__doc__)
    revit = {
        wall.Tag: association(ifcopenshell.util.element.get_material(wall))
        for wall in ifcopenshell.open(argv[1]).by_type("IfcWall")
    }
    for path in argv[2:]:
        classes = collections.Counter()
        rule = collections.Counter()
        for wall in ifcopenshell.open(path).by_type("IfcWall"):
            want = revit.get(wall.Tag)
            if want is None:
                continue
            got = association(ifcopenshell.util.element.get_material(wall))
            if got[0] == "layer set":
                classes[f"layer set (Revit's: {want[0]})"] += 1
                layers = got[1]
                if len(layers) == 1:
                    rule["one layer: Revit's material of that name" if want == ("material", layers) else "one layer: other"] += 1
                elif want[0] == "constituent set":
                    rule["several: Revit's constituents, names numbered" if want[1:] == (layers, numbered(layers)) else "several: other"] += 1
                else:
                    rule["several: Revit writes no constituent set"] += 1
            elif got[0] == "none":
                classes[f"none (Revit's: {want[0]})"] += 1
            elif got == want:
                classes[f"{got[0]}, Revit's, names too"] += 1
            elif got[:2] == want[:2]:
                classes[f"{got[0]}, Revit's materials, names differ"] += 1
            else:
                classes[f"{got[0]}, Revit's: {want[0]}, differs"] += 1
        print(path)
        for k, v in sorted(classes.items(), key=lambda x: -x[1]):
            print(f"  {v:5d} {k}")
        for k, v in sorted(rule.items(), key=lambda x: -x[1]):
            print(f"  rule on layer-set walls: {v:5d} {k}")


if __name__ == "__main__":
    main(sys.argv)

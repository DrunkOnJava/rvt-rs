#!/usr/bin/env python3
"""Compare each element's materials in an rvt-rs IFC with Revit's own IFC (RE-82).

Research tool for `reports/element-framing/RE-82-type-material-maps.md`.
For every element with a `Tag` in both files (openings excluded), reads the
material names IfcOpenShell resolves for it (`IfcMaterial`, or the
materials of an `IfcMaterialLayerSet(Usage)`, `IfcMaterialConstituentSet`,
`IfcMaterialProfileSet` or `IfcMaterialList`) and counts the elements
Revit gives materials, those rvt-rs gives materials, and how many of those
are the same set.

Needs IfcOpenShell 0.8.5 (`pip install ifcopenshell==0.8.5`).

Usage:

    python3 tools/re/element_materials_vs_ifc.py <rvt-rs.ifc> <revit.ifc>
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.util.element as element_util


def material_names(material):
    if material is None:
        return set()
    if material.is_a("IfcMaterial"):
        return {material.Name}
    if material.is_a("IfcMaterialLayerSet"):
        return {layer.Material.Name for layer in material.MaterialLayers if layer.Material}
    if material.is_a("IfcMaterialConstituentSet"):
        return {c.Material.Name for c in material.MaterialConstituents or [] if c.Material}
    if material.is_a("IfcMaterialProfileSet"):
        return {p.Material.Name for p in material.MaterialProfiles if p.Material}
    if material.is_a("IfcMaterialList"):
        return {m.Name for m in material.Materials}
    return set()


def by_tag(path):
    model = ifcopenshell.open(path)
    out = {}
    for element in model.by_type("IfcElement"):
        if element.is_a("IfcOpeningElement") or not element.Tag:
            continue
        material = element_util.get_material(element, should_skip_usage=True)
        out.setdefault(element.Tag, (element.is_a(), material_names(material)))
    return out


def main():
    ours, revit = by_tag(sys.argv[1]), by_tag(sys.argv[2])
    with_revit = with_ours = same = 0
    differ = collections.Counter()
    missing = collections.Counter()
    for tag, (entity, theirs) in revit.items():
        if not theirs or tag not in ours:
            continue
        with_revit += 1
        mine = ours[tag][1]
        if not mine:
            missing[entity] += 1
            continue
        with_ours += 1
        if mine == theirs:
            same += 1
        else:
            differ[entity] += 1
    print(f"elements Revit gives materials: {with_revit}")
    print(f"of those, rvt-rs gives materials: {with_ours}, the same set: {same}")
    print(f"different sets by Revit entity: {dict(differ.most_common())}")
    print(f"no materials from rvt-rs, by Revit entity: {dict(missing.most_common())}")


if __name__ == "__main__":
    main()

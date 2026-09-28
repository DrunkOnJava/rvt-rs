#!/usr/bin/env python3
"""Compare the shading colour and transparency of each material in an rvt-rs
IFC with Revit's own IFC export of the same file (RE-116).

Revit's export reaches a material's surface style through its
`IfcMaterialDefinitionRepresentation`; rvt-rs names each material's
`IfcSurfaceStyle` after the material. Both are read, by material name, and
every material Revit's export styles is counted as having the same colour
and transparency in rvt-rs's, a different one, or none.

Usage:
    python3 tools/re/material_colours_vs_ifc.py OURS.ifc REVIT.ifc
"""

import sys

import ifcopenshell


def shading(style):
    for item in style.Styles:
        if item.is_a("IfcSurfaceStyleShading"):
            colour = item.SurfaceColour
            return (
                round(colour.Red * 255),
                round(colour.Green * 255),
                round(colour.Blue * 255),
                round(float(item.Transparency or 0), 3),
            )
    return None


def colours(path):
    model = ifcopenshell.open(path)
    out = {}
    for material in model.by_type("IfcMaterial"):
        for definition in material.HasRepresentation or []:
            for representation in definition.Representations:
                for item in representation.Items:
                    for assignment in item.Styles:
                        styles = (
                            [assignment]
                            if assignment.is_a("IfcSurfaceStyle")
                            else getattr(assignment, "Styles", None) or []
                        )
                        for style in styles:
                            if style.is_a("IfcSurfaceStyle") and shading(style):
                                out.setdefault(material.Name, shading(style))
    for style in model.by_type("IfcSurfaceStyle"):
        if style.Name and shading(style):
            out.setdefault(style.Name, shading(style))
    return out, len(model.by_type("IfcMaterial"))


def main():
    ours, ours_count = colours(sys.argv[1])
    revit, revit_count = colours(sys.argv[2])
    same = [name for name in revit if ours.get(name) == revit[name]]
    different = [(name, ours[name], revit[name]) for name in revit if name in ours and ours[name] != revit[name]]
    missing = [name for name in revit if name not in ours]
    print(f"IfcMaterial: rvt-rs {ours_count}, Revit {revit_count}")
    print(f"Revit's styled materials: {len(revit)}; same colour and transparency: {len(same)}")
    print(f"different: {different}")
    print(f"unstyled or absent in rvt-rs: {missing}")


if __name__ == "__main__":
    main()

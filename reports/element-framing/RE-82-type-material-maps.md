# RE-82 — Family instances' materials from their type's geometry-material map

**Date:** 2026-09-27
**Issue:** #355 (the element-to-material link for family instances)
**Probe:** `examples/probe_re82_type_material_maps.rs`; scoring `tools/re/element_materials_vs_ifc.py`

**Result:** positive. Family instances export their materials as Revit's export writes them, from a map in their type's parameter value block.

## The map

A family type's value block (RE-77) holds a counted map, `u32 n` followed by `n × (u32 key · u64 material ElementId)`, whose every value is a material. On a Snowdon Towers window type it reads Wood - Stained three times and Glass once: the materials of its geometry. The same block names "Clad - White" elsewhere, a material a parameter offers but the geometry does not use, and Revit's export leaves that out. The map's keys are not decoded.

A first attempt took every material ElementId in the type's value block. It contained Revit's materials for 98.5% of elements but equalled them for only 86%: windows, doors, lights and curtain panels picked up their alternative materials. Restricting to the map fixed those.

Walls, floors, roofs and ceilings have no such map; they keep their layer sets (RE-58).

## Measured

Each instance takes its type's map materials (RE-38's type join), exported as an unnamed `IfcMaterialConstituentSet` with one constituent per material, named after it, in the category "Materials", in name order, as Revit writes family instances. Elements with a layer set, profile set or single material keep that.

`tools/re/element_materials_vs_ifc.py`, rvt-rs export against Revit's own export, elements Revit gives materials:

| file | release | with materials, before | with materials, now | the same set as Revit's | different |
|---|---|---:|---:|---:|---:|
| Snowdon Towers (local only) | 2024 | 1,203 | 2,574 | 2,533 | 41 |
| Core Interior (slim export) | 2024 | 88 | 226 | 220 | 6 |
| RE1 Architecture | 2025 | 15 | 58 | 58 | 0 |

The differences:

- **21 counter tops** (Snowdon, `IfcFurniture`): the map also carries the materials of an appliance nested in the counter, which Revit gives to the appliance.
- **17 planting elements**: Revit writes one material named `<Unnamed>`; the map names Tree Canopy and Tree Trunk.
- **6 windows** (Core Interior): Revit's set has a sill material ("Wood_Walnut black") and names the frame "SH_Aluminum, Anodized Black" where the map names "Aluminum_Black". This is likely also a nested component.
- 3 single elements on Snowdon.

## Not claimed

- Instance-level material overrides, and elements whose material is set only by an instance parameter.
- Materials of nested components as their own elements.
- Colours of materials that only family types use: `IfcMaterial` rows added for them carry no surface style unless the material's appearance was read (RE-53).
- Revit 2023 (#421): the value-block and name layouts are not measured there.

# RE-88 — Wall materials where the body is not a layer set

**Date:** 2026-09-28
**Issues:** #355, #358
**Result:**
- Revit's IFC4 export associates a wall's materials by its type's layers alone: a type of one layer gives the wall that layer's `IfcMaterial`, and a type of several gives an `IfcMaterialConstituentSet`. The set has one constituent per layer, exterior first, each named after its material and in the category "Materials". A material's k-th occurrence is named with " (k)" after it; " (2)", " (3)" and " (4)" all occur. On Snowdon Towers this holds on every wall rvt-rs writes a layer set for: 143 of 143 single materials have the same name, and 823 of 824 constituent sets have rvt-rs's layer materials in the same order under those names.
- rvt-rs writes an `IfcMaterialLayerSetUsage` where a wall's body fits its layers (RE-58), and wrote nothing where it does not. Such walls are tapered (RE-86), curved (RE-75), cut or profiled. Where their layers are read, they now take the association Revit's export gives them.

| Snowdon Towers' 111 walls without a material on main | main | RE-88 |
|---|---:|---:|
| Revit's constituent set: same materials in the same order, same names | 0 | 57 of 59 |
| Revit's single material: same name | 0 | 15 of 52 |
| a different association from Revit's | 0 | 0 |

- The other 39 have no layers read (21 face walls and walls with no orientation, and 2 Revit gives a constituent set), a layer with no material name (15), or are an in-place family (1).
- Revit writes the 15 layers with no material name as "Default Wall", presumably the Walls category's material. That is not read, so those walls stay without one.

-----

## 1. The rule

`tools/re/wall_materials_vs_ifc.py` compares each wall's material association with Revit's. On the 967 Snowdon walls rvt-rs writes a layer set for, the rule is applied to rvt-rs's layers and compared with Revit's association:

```text
rule on layer-set walls:   823 several: Revit's constituents, names numbered
rule on layer-set walls:   143 one layer: Revit's material of that name
rule on layer-set walls:     1 several: other
```

The one exception has a layer rvt-rs reads without a material, which Revit writes as "Default Wall". All 883 Revit walls with a constituent set have a type carrying the same one. The rule does not depend on the body.

## 2. The association

`ifc::layer_materials_without_layer_sets` runs after RE-58's layer sets and RE-82's family materials. It gives an element with read layers and no other association either of these:
- the one layer's material;
- a constituent set of its layers, exterior first, with a material's k-th constituent named " (k)" after it.

It leaves alone a floor's, roof's or ceiling's stacked layers (not measured here), and any type with a layer whose material name is not read. `MaterialConstituentSet` gains `names`, one per constituent; RE-82's sets leave it empty and are written as before. In the GLB, a wall with a single material now takes its colour, as a family instance with one does (#355).

## 3. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc (local only) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |

```bash
./target/ci/rvt-ifc MODEL.rvt -o model.ifc
python3 tools/re/wall_materials_vs_ifc.py REVIT_EXPORT.ifc main.ifc model.ifc
```

- **Snowdon Towers:** only the 72 walls' material associations change (57 constituent sets, 15 materials). Every product's placement and shape is unchanged.
- **Snowdon Towers structural, Core Interior, the four RE1 models, Einhoven and the MIT house (2024 and 2025):** IFC byte-identical to main's apart from the timestamp.

## 4. Open

- Face walls (16 on Snowdon) and walls with no orientation read have no layers, so no material.
- A layer that takes its category's material: Revit writes "Default Wall" for it, and where that name comes from is not read.
- Floors, roofs and ceilings whose body is not a layer set (stacked layers) are not measured.

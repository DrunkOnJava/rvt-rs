# RE-57 — Floors, roofs and ceilings drawn as their layers

**Date:** 2026-09-23
**Result:**
- rvt-rs's glTF export and viewer now draw a Revit 2024 or 2025 floor, building pad, flat roof or ceiling as its type's layers:
  - stacked top first;
  - each in its material's shading colour;
  - wherever the layers add up to its record box's height.

  The layers (RE-53, RE-56) and colours were already read; walls were the only elements drawn in them.
- Scored end to end against Revit's own IFC4 exports:

| file | layered in the GLB | compared | layer bottoms and tops within 0.001 ft | layer colours equal to Revit's |
|---|---:|---:|---:|---:|
| Snowdon Towers (2024) | 246 | 236 | 236 | 358 of 358 |
| 2024_Core_Interior | 68 | 68 | 68 | 69 of 69 |
| RE1 Architecture (2025) | 8 | 0 (Revit styles each body with one material) | — | 8 of 8, in order |

- The IFC output is unchanged.

**Probe:** `examples/probe_re53_wall_layers.rs` now also counts floors, roofs and ceilings drawn in layers:

| file | floors, roofs and ceilings drawn in layers |
|---|---:|
| Snowdon | 249 |
| Core Interior | 100 |
| RE1 Architecture | 8 |
| the tutorial house | 8 |

**End-to-end check:** `tools/re/slab_layers_vs_ifc.py`, the stacked counterpart of RE-53's wall scorer. It reads the GLB `rvt-gltf` writes and Revit's IFC4 export of the same file. For each layer whose material occurs once in its element, it compares the layer's bottom and top with the heights of the faces Revit styles with that material. It also compares colours.

-----

## 1. Which layers are a slab's

`partition_schema_mvp::attach_slab_layers` looks at every floor, building pad, roof and ceiling with a type (RE-44). It gives the element its type's layers where they add up to its record box's height within 1e-4 ft. The plate is then exactly its layers. A roof drawn along its slope (RE-56) is left out.

Where the type's layers are read, they add up to the box's height on:

| file | ceilings | floors | roofs | building pads |
|---|---:|---:|---:|---:|
| Snowdon Towers | 68 of 68 | 174 of 182 | 3 of 20 | — |
| Core Interior | — | 99 of 99 | — | 1 of 1 |
| RE1 Architecture | 6 of 6 | 2 of 2 | — | — |
| the tutorial house | — | 4 of 4 | 3 of 4 | 1 of 1 |

The ones that do not add up stay whole:
- Snowdon's 17 roofs are tapered-insulation roofs.
- Its 8 floors have boxes taller than their layers, for example 0.833 ft against 0.333 ft.
- The tutorial house's other roof is RE-56's shed.

A type's layers run top first: the first layer is the top of the plate.
- On Snowdon and Core Interior, Revit's IFC body puts each layer's material at exactly the heights rvt-rs draws it.
- On RE1, the export's layer-set usage places its first layer at the top of the ceiling, and the oak floor finish sits on its sleepers.

## 2. Drawing them

`IfcModel::element_layers` carries a slab's layers with `stacked` set. `body_geometry::stacked_extrusion_meshes` cuts its extruded body (sketch outline or box) into one prism per layer, from its top down. The widths must add up to its height within 0.02 ft. `gltf.rs` writes one node per layer, all naming the element, as for walls.

The viewer's File status reports all layered elements under Materials ("layers and colours read for N elements"). The export diagnostics' `exported.layered_element_count` counts them too.

## 3. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |
| 2024_Core_Interior.rvt | `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014` |
| IFC Exports/2024_Core_Interior_slim.ifc | `bfdf36ffb0bb768f3409d818403990e64d4c262c6780603be87f8077387ad86d` |
| RE1-Architecture.rvt | `4a78bdd68e7f4dd7806060db07c222da9a810e20b3f303c17a443294b91c8ce4` |
| RE1-Architecture.ifc | `a9b5d36677aa6a8bb91b77d8bc354ed9028a7ca3e9e2491a47ba14cfd8e26200` |

```bash
./target/ci/rvt-gltf MODEL.rvt -o model.glb
python3 tools/re/slab_layers_vs_ifc.py model.glb REVIT_EXPORT.ifc
```

- **Snowdon.** Of the 246 layered floors, roofs and ceilings in the GLB, 239 are in Revit's export and IfcOpenShell meshes 236. All 236 have every layer's bottom and top within 1.4e-6 ft of Revit's, and all 358 compared layer colours equal Revit's.
- **Core Interior.** All 68 floors in the slim export match to 3.4e-6 ft, with 69 of 69 colours equal.
- **RE1.** Revit's 2025 export styles each of its 8 floors' and ceilings' bodies with a single material over its whole thickness, so the layers' heights cannot be measured there. Their 8 colours equal Revit's in Revit's layer order.

## 4. What this does not do

- Slabs whose layers do not add up to their box: tapered insulation, and floors whose box is taller than their layers. RE-56's sloped roof also stays one body.
- Layers that wrap at a slab's edges.
- Material names (#355). So the IFC export writes no layer sets.

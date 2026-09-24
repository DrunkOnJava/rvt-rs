# RE-53 — Walls drawn as their layers, each in its material's colour

**Date:** 2026-09-23
**Result:**
- **Positive.** A material's serialised object holds its shading colour and transparency. They equal the `IfcSurfaceStyleRendering` Revit's own IFC4 export gives the same material on 123 of 123 materials matched by name across five files.
- **Positive.** A wall, floor, roof or ceiling type's own data holds its compound structure: a layer count and one 37-byte record per layer, exterior (or top) first, with the layer's width, material, deck profile and function. Each type measured gives Revit's layers.
- **Positive.** A wall's data holds its location line and a flip flag. The flag says whether the wall's exterior lies to the right or the left of its location line's direction. It holds on all 416 Snowdon Towers walls whose two outer layers Revit's IFC places measurably apart.
- rvt-rs's glTF export (`rvt-gltf`, the viewer) now draws a Revit 2024 wall as its layers, each in its material's colour, where the layers add up to its body's thickness.
  - On Snowdon Towers that is 883 of the 1,054 walls whose layers are read.
  - Scored end to end, every layer centre is within 0.001 ft of Revit's on 851 of 862 comparable walls, and within 0.126 ft on all of them. The 11 others are one soffit wall type whose stud layer Revit cuts back.
  - Every compared layer's colour equals Revit's surface style colour: 1,368 of 1,368.
- The IFC output is unchanged. Writing `IfcMaterialLayerSet`s needs material names, which are not read (#355).

**Probe:** `examples/probe_re53_wall_layers.rs` exports a file as the viewer does and counts the walls drawn in layers and why the others are not (`--verbose` lists each fallback's body and layer thickness). `--json` writes each drawn wall's layer spans.

**End-to-end check:** `tools/re/wall_layers_vs_ifc.py` reads the GLB `rvt-gltf` writes and Revit's IFC4 export of the same file, and compares each wall's layers, position and colour, with IfcOpenShell meshing Revit's side. Commands, input hashes and output are in §6.

**Oracles:**
- Snowdon Towers Sample Architectural, Revit 2024 (local only, no licence), with Revit's IFC4 export of it (`bldrs-ai/test-models`). Revit writes each wall with an `IfcMaterialConstituentSet`, one constituent per layer in layer order. Its body's faces carry the surface style of the layer they belong to.
- For the colour and layer readings also: 2024_Core_Interior (Revit 2024), RE1 Architecture (Revit 2025), and Projeto1 and teste_export_2025 (Revit 2025, local only), each with Revit's IFC4 export.
- No licensed file in CI has a Revit 2024 wall with a coloured layer. Core Interior's wall types are single layers by category, RE1 is Revit 2025 (§5) and Einhoven is Revit 2023. So CI does not measure this.

-----

## 1. A material's colour

A Revit material is an object `01 00 00 00 · u64 id` whose class tag sits 0x47 bytes past the id: `28 0a` on Revit 2024 and `6b 0a` on Revit 2025.
- Some of these objects open with the element-data header (RE-44), which ends in the same `01 00 00 00 · u64 id`.
- Others, the materials families bring with them, sit inside another element's data.

Found by the tag, they number exactly the `OST_Materials` records on every file measured.

The first frame of this shape after the id holds the material's shading:

```text
ff ff ff ff ff ff ff ff
f32   transparency, 0 to 1
f32   0.5 on every material measured
4 ×   u32 COLORREF   pattern colours
u32   COLORREF       shading colour, r g b 00
u32   shininess, 64 by default
```

Shading colour and transparency equal the `SurfaceColour` and `Transparency` of the `IfcSurfaceStyleRendering` Revit's export gives the material on 123 of 123 materials across Snowdon Towers, Core Interior, RE1 Architecture, Projeto1 and teste_export_2025. The material was matched to its style by name, so this covers only the materials whose name reads (#355).

§6 measures the colours a second way, with no names. A wall's layer names its material by ElementId, and Revit's IFC styles the same layer's faces.

rvt-rs does not read the name. On the materials families bring, the name field is not identified yet.

`partition_materials::scan_material_appearances` reads the frame within 0x2000 bytes of the id.

## 2. A host type's layers

A wall, floor, roof or ceiling type's own element data (RE-44's header and ElementId) carries a `u32` layer count and one record per layer, exterior or top first:

```text
+0   f64  width, feet (0 only on a membrane)
+8   u64  material ElementId, ff × 8 = by category
+16  u64  deck profile ElementId, ff × 8 = none
+24  u32  function: 1 structure, 2 substrate, 3 thermal / air,
          4 finish 1, 5 finish 2, 100 membrane, 200 structural deck
+28  9 bytes, not read
```

The count is framed one of two ways:
- by `ff ff ff ff` and a per-release tag, `a6 10` on Revit 2024 and `0e 11` on Revit 2025. On 2024 this follows the type's name directly.
- by `u32 k`, then `k` × `2d 00`, then `u32 0`, on Revit 2025 floors and ceilings.

A candidate is kept only if every material it names is an `OST_Materials` record or "by category", and every width is finite and non-negative. Against the materials Revit's IFC4 export gives the same types:
- Snowdon Towers: 42 of 42 types give Revit's constituent sequence, once the 0-width membranes Revit leaves out are dropped.
- Core Interior: 4 of 4 types, every layer by category.
- RE1 Architecture: the wall, both floors and the ceiling give Revit's layer widths and materials.

`partition_compound_structure::scan_type_layers` reads them.

## 3. Which side is the exterior

A wall's data carries, after `ff ff ff ff 01 00 00 00`, three `u32`:
- its location line (0 to 5);
- a word of 0 to 2;
- a flip flag.

The wall's direction is its location line, the bounded line record RE-49 reads for beams (`04 00 08 01`). It runs the way Revit's IFC `Axis` does. With the flag set, the wall's exterior (its first layer) lies to the right of that direction: the normal (dy, −dx). Clear, it lies to the left: (−dy, dx).

This was measured against Revit's IFC4 per-layer solids. The first and last layers' centres were compared across each wall.
- On all 416 Snowdon walls where those centres are measurably apart, the flag gives the side Revit puts the first layer on.
- On the one other wall, the two outer layers are 0.03 ft apart, too close to tell.

An earlier search against the 2027 VIM of Snowdon found no flag at a fixed offset. That search is superseded: the edition drift made its reference side unreliable.

`partition_compound_structure::scan_wall_flips` reads the flag. Location lines are read on Revit 2024 only (RE-49), so walls are drawn in layers on 2024 files only.

## 4. Drawing the layers

`partition_schema_mvp::attach_wall_layers` gives each wall its exterior normal and its type's layers (membranes dropped), each layer with its material's colour and transparency, or "by category". They reach `IfcModel::element_layers`.

`body_geometry::layered_extrusion_meshes` cuts the wall's extruded body into bands across that normal:
- The exterior face is the body's outline's furthest point along the normal.
- Each band is the outline clipped between two lines across it (Sutherland–Hodgman) and extruded as the body is.

It draws the layers only if the body is one outline with no holes and the layers add up to its thickness along the normal within 0.02 ft. Otherwise the wall is drawn whole, as before.

`gltf.rs` writes a layered wall as one node per layer, all carrying the wall's `entityIndex`, so picking, selection and category visibility treat it as one element. Each layer's material is its colour, or the wall's category colour for a layer by category.

The viewer's File status reports the count under Materials ("layers and colours read for N walls"). The export diagnostics carry it as `exported.layered_element_count`.

## 5. What is drawn on each file

| file | release | walls with layers read | drawn in layers | layers do not match the body |
|---|---|---:|---:|---:|
| Snowdon Towers Architectural | 2024 | 1,054 | 889 | 165 |
| 2024_Core_Interior | 2024 | 315 | 315 | 0 |
| RE1 Architecture, Projeto1, teste_export_2025 | 2025 | 0 | 0 | 0 |
| Einhoven, modelo_bim, Exemplo_data | 2023 | 0 | 0 | 0 |

On Core Interior every wall type is one layer by category, so the drawing does not change.

Of Snowdon's 889:
- 883 are drawn in at least one material's colour: 764 with two or more layers, 119 with one.
- The other 6 are one layer by category.

### The 165 not drawn

The 165 are walls whose body, as rvt-rs builds it, is not as thick as the layers. The most common cases:

| walls | example | rvt-rs body | layers | Revit's IFC body |
|---:|---|---:|---:|---:|
| 18 | 2083876 Soffit - Beam Wrap (Both Sides) | 0.771 ft | 0.875 ft | 0.427 ft |
| 13 | 2085017 Soffit - Beam Wrap 9 1/4" (One Side) | 0.667 ft | 0.771 ft | 0.593 ft |
| 9 | 1456456 Retaining - Split Face CMU | 0.802 ft | 0.635 ft | 0.635 ft |
| 7 | 2060104 Generic - 4" | 7.119 ft | 0.333 ft | tessellated, 22.2 ft |
| 3 each | 2140499/500/501 Exterior - 12 5/8" Rainscreen | 2.28 to 2.34 ft | 1.052 ft | 1.052 ft |

Where Revit's body is as thick as the layers, the layers are right and rvt-rs's body is not: 1456456 and the rainscreen walls. Most of the rest are soffit wraps and profiled walls whose body Revit cuts or tessellates. They are left whole rather than cut into layers that would not fill them.

### Revit 2025

Location lines are gated to Revit 2024. Opening that gate on 2025 as an experiment:
- RE1 Architecture draws 7 walls in layers, but every layer's material appears more than once in its wall, so Revit's styles cannot place them and the side cannot be scored.
- teste_export_2025's 6 walls have bodies 24 to 77 ft across and fall back.
- Projeto1's one wall is one layer by category.

With no 2025 wall whose side can be measured, the gate stays at 2024.

## 6. End-to-end measurement

Inputs:

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc (Revit 24.0.20.20) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |

Commands (rvt-rs at this change, IfcOpenShell 0.8.5):

```bash
cargo build --profile ci --bin rvt-gltf
./target/ci/rvt-gltf "Snowdon Towers Sample Architectural.rvt" -o snowdon.glb
python3 tools/re/wall_layers_vs_ifc.py snowdon.glb "Snowdon Towers Sample Architectural_IFC4.ifc"
```

`rvt-gltf` takes 8.8 s and writes 5,287,140 bytes (6,105 building elements). The scorer prints:

```text
883 walls drawn in layers in the GLB
    119  of them one layer
    883  drawn in layers in the GLB and present in the IFC
    862  compared
    743  compared, two or more layers
    119  compared, one layer
     21  no layer with a unique material
  layer centres within 0.001 ft: 851 of 862
  layer centres within 0.01 ft: 851 of 862
  layer centres within 0.05 ft: 860 of 862
  layer centres within 0.25 ft: 862 of 862
  median 3.33e-07 ft, max 0.1261 ft
compared layers' colours against Revit's surface styles:
   1368  same
```

For each wall present in both files:
- The scorer takes the direction across the wall from Revit's own wall placement, turned into the model's frame by the site placement Revit's export states.
- It orients that direction from rvt-rs's last layer to its first.
- It compares each layer's centre along it wherever the layer's material occurs once in the wall.

A wall drawn with its exterior on the wrong side would put its exterior layer where Revit puts its interior one. So the 743 walls of two or more layers also check the flip.

The 11 walls beyond 0.001 ft are all type "Soffit - Beam Wrap 9 1/4" (One Side)":
- 1463499 and 1463500 are off by 0.126 ft; 2028209–2028211, 2028273, 2078852, 2078865, 2078876, 2078889 and 2147456 by 0.047 ft.
- Their two gypsum layers match Revit's.
- The stud layer is 0.667 ft thick in the type and in rvt-rs, but Revit's stud layer solid is 0.41 to 0.57 ft, cut back where the soffit wraps a beam.

The 21 not compared have no layer whose material is unique in the wall.

The viewer check is the same GLB path through `rvt_bg.wasm`:
- Snowdon loads with 1,054 walls reported under Materials.
- Wall 735573 (six layers, 0.854 ft) shows its six bands at its top edge in order: f0f1ec, ffffff, e7e7e7, the 0.052 ft 787878 band, the 0.302 ft c7c7c7 band and dddddb.
- Selecting it highlights all six as one element.

## 7. What this does not do

- Floors, roofs and ceilings: their layers are read (§2) but not drawn. A slab's body is not yet cut into its layers.
- Revit 2025 and earlier releases (§5).
- Material names (#355). So the IFC export writes no layer set and no surface style from these colours.
- The 165 walls whose body rvt-rs builds differently from their layers (§5, #358). Their bodies are the defect, and they are drawn whole.
- Layers that wrap at a wall's ends and openings, and the core boundary: layers are drawn as parallel bands of the body as rvt-rs builds it.

# RE-116 — Revit 2023 materials' shading colours and names, and the material list

**Date:** 2026-09-28
**Issues:** #421 (Revit 2023), #355 (materials); follows RE-53, RE-58, RE-113, RE-114 and RE-115
**Artefacts:** `255ribeiro/intro_ifc` `Exemplo_data.rvt` and `modelo_bim.rvt` (Revit 2023, local only, no licence), each with Revit's own IFC4 export; `Revit_IFC5_Einhoven.rvt` (Revit 2023, MIT, magnetar-io), with no export.
**Result:** a Revit 2023 material keeps its shading colour, transparency and shininess in its object, as 2024's does, in a 2023 frame, and ends its name as 2024's does (RE-58). The 2023 export's material list is now the file's own materials, named and coloured, where it held partition display strings before.

**Colours.** Against the surface styles Revit's export writes:

| | Exemplo_data | modelo_bim |
|---|---:|---:|
| Revit's styled materials | 11 | 10 |
| with Revit's colour and transparency | 11 | 9 |
| with a different colour | 0 | 0 |

The one other is modelo_bim's `Concreto - Concreto moldado in loco`, which is in no material object; its footings keep no material.

**The material list.**

| | Exemplo_data | modelo_bim |
|---|---:|---:|
| RE-115 (partition display strings, as on any unmeasured release) | 155 | 146 |
| RE-116 (the file's own materials) | 114 | 108 |

Einhoven's list, 42 display strings before, is its 36 materials (`Default`, `Default Wall`, `Glass`, …), every one named and coloured; its project-count baseline moves from 42 to 36.

The strings were largely English library names: `Beechwood - Galliano`, `Brick & Tile Plazas`.

**The GLB.** Elements drawn in one material take its colour, as on 2024: modelo_bim's beams and columns take `Concreto, Moldado in loco, cinza` (192, 192, 192), and its walls' layer `Unidades de alvenaria de concreto` (181, 181, 181).

Every 2024 and 2025 file is byte-identical: Core Interior, Snowdon Towers Architectural, RE1 Architecture, Projeto1 and teste_export_2025.

-----

## 1. The frame

2024's frame (RE-53) is `ff × 8 · f32 transparency · f32 0.5 · 4 × u32 COLORREF · u32 COLORREF shading · u32 shininess`. On 2023 each of the four pattern slots is eight bytes, `u32` fill-pattern ElementId (`ff` × 4 when unset) and `u32` COLORREF:

```text
f32  transparency (0.9 on Vidro, glass)
f32  0.5
4 ×  u32 pattern ElementId · u32 COLORREF
u32  COLORREF shading colour, r g b 00
u32  shininess (128; 64 on Telhado padrão, 12 on Vidro, as Revit's IFC writes)
```

A material that opens its own element data frames this with `ff ff ff ff 9b 0b`; a family's own material, inside another element's data, does not. So the frame is recognised by a second float of exactly 0.5, slots holding an unset or plausible id, and zero high bytes in every COLORREF (`partition_materials::frame_at_2023`). The first frame after the material tag is the material's.

## 2. Names

A 2023 material's name is now read by the first of these that holds one, the order RE-58 reads a 2024 name in:

1. **Its own name field.** Right after the class tag, two to four zero `u32`s, then `u32 n` and the name, unless a BuiltInParameter id follows the slot: `00 × 8 · 0c 00 00 00 · "Default Wall"` on Einhoven.
2. **The name ending right before `ff ff ff ff eb 0b`.**
3. **Its `-1001203` entry.**
4. **Its element data's first framed string.**

A "name" of private-use or specials-block units is never one.

RE-113 read only the last two. On one material, 338378 `Revestimento – Branco`, that string is an appearance asset's file name, `assetlibrary_base.fbx`, which several materials share, so the name was dropped. A 2023 material's name ends right before `ff ff ff ff eb 0b`, as RE-58 ends a 2024 name before `ff ff ff ff 17 0c` and a 2025 one before `ff ff ff ff 6b 0c`: the terminator moves with the release, as the name frame tag does (`0x013f`, `0x0149`, `0x015c`).

**What the new order changes.** Every name read before is unchanged on both ribeiro files, and each gains 30:
- the three materials named by their terminated name alone, `Revestimento – Branco`, `Aço, acabamento de pintura, marfim, fosco` and `Material de Parede (1)`;
- analytical and system materials named by their own name field (`Zonas do sistema`, `Superfície analítica – Paredes externas`, …).

On Einhoven the own name field names all 36. So:
- Exemplo_data lists 114 named materials (81 in RE-113), modelo_bim 108 (78);
- its roller door (`Porta-Rolante-Pendurada`), which RE-113 left without materials, has Revit's.
- Exemplo_data's elements with Revit's material set are now 36 of 37 (35 in RE-115).

## 3. What changed

- **The material list.** `partition_schema_mvp::materials_from_records` now builds 2023's list from RE-113's material objects and names, with these colours. Before, 2023 fell back to the partition display strings, as a release without measured material records does.
- **Layer colours.** Wall and slab layers (RE-114, RE-115) take their material's colour, so the GLB draws them in it.

## 4. End-to-end measurement

| file | sha256 |
|---|---|
| Exemplo_data.rvt | `c647c521262860bfa765fefda0d4c7542c9164a389d6af1c08b3d31d142588e2` |
| Exemplo_data.ifc | `42fdb8b0b540ebb993064e347ddf4a3e50d329f5cd8f45d0b6c73ee7a7da8c4a` |
| modelo_bim.rvt | `e06107f54481f408995ddf8f66093f1e4c4bc576ac0a3ff186edc4d32e21ff62` |
| modelo_bim.ifc | `58a6aa2872a818a43ed383e833a3a88ec768ef20ef75658bcbd9f7e846fab47f` |

```bash
./target/ci/rvt-ifc modelo_bim.rvt -o modelo.ifc
python3 tools/re/material_colours_vs_ifc.py modelo.ifc modelo_bim.ifc
./target/ci/rvt-gltf modelo_bim.rvt -o modelo.glb
```

# RE-114 — Revit 2023 walls' lines and orientation, and material layer sets

**Date:** 2026-09-28
**Issues:** #421 (Revit 2023), #355 (materials); follows RE-49, RE-53, RE-57, RE-112 and RE-113
**Artefacts (local only, no licence):** `255ribeiro/intro_ifc` `Exemplo_data.rvt` and `modelo_bim.rvt` (Revit 2023), each with Revit's own IFC4 export.
**Result:** a Revit 2023 wall's data holds its location line and its orientation as 2024's does. With RE-112's layers and RE-113's material names, 2023 walls and floors are now written with an `IfcMaterialLayerSetUsage` where their layers' materials are named.

| modelo_bim | before | RE-114 | Revit's export |
|---|---|---|---|
| 4 walls | no material | `Unidades de alvenaria de concreto`, 0.14 m | the same |
| floor 337638 | no material | `Concreto, Moldado in loco`, 0.21 m | the same, then `Deck de metal`, 0.0 m |

Scored with `tools/re/element_materials_vs_ifc.py`, modelo_bim's elements with Revit's own material set go from 27 of 37 (RE-113) to 31 of 37.

Every 2024 and 2025 file is byte-identical: Core Interior, Snowdon Towers Architectural, RE1 Architecture, Projeto1 and teste_export_2025.

-----

## 1. One element-data layout per release

Every reader of an element's own data found it by the release's header followed by a `u64` ElementId: RE-49's lines, RE-53's orientations and RE-53's layers. On 2023 the header ends `c0 02 01 00 00 00` and the id is a `u32` followed by four more bytes (RE-111), so the data proper starts 8 bytes past the id on every release. `partition_names::ElementDataLayout` carries the header and the id width, and the line and orientation readers now go through it.

## 2. Lines and orientation

Read that way, all 21 walls of the two projects have a bounded line and an orientation.
- **The line** is the wall's centreline before joins. On Exemplo_data each of its ends lies on the end of Revit's `Axis` or exactly 0.1 m (half the 200 mm wall) from it, where Revit runs the axis on into, or stops it short of, the wall it joins. Of the 17 walls, 1 is exact, 1 has both ends extended, 7 have both trimmed, and 8 have one of each. The record box already spans those joins.
- **The orientation.** 4 of Exemplo_data's walls have the flip flag set. The flag cannot be checked against Revit's export here: every wall on both files has a single layer, so its exterior side does not move a layer.
- **The cross-section word** is 1 on every wall, a vertical wall as on 2024.

## 3. What is written

**Walls.**
- Each gets RE-53's layers and exterior side.
- Its body stays the record box. On 2024 a wall with a line is drawn along it and trimmed by RE-70's joins; 2023's join lists are not read, and a line without them would lose the corners the box already has.

**Floors and roofs.** Each gets RE-57's layers.

**Where layer sets are written.** An `IfcMaterialLayerSetUsage` is written where a layer's material is named, as on 2024. That leaves:
- **modelo_bim's walls:** layer and thickness equal Revit's. The usage axes differ from Revit's only by frame: Revit places each wall along its axis, while a 2023 record box is placed along the model's axes.
- **modelo_bim's floor:** its concrete layer. Revit also writes the metal deck at 0.0 m. rvt-rs leaves zero-width layers out of layer sets on every release, and none of Revit's 2024 and 2025 exports here writes one, so that stays.
- **Unnamed layers.** Exemplo_data's walls and floor, and modelo_bim's roof, take their category's material, which 2023 does not read yet (RE-91 is 2024 only). Revit's export names them `Material pré-definido de parede`, `Material pré-definido de piso` and `Telhado padrão`.

## 4. End-to-end measurement

| file | sha256 |
|---|---|
| modelo_bim.rvt | `e06107f54481f408995ddf8f66093f1e4c4bc576ac0a3ff186edc4d32e21ff62` |
| modelo_bim.ifc | `58a6aa2872a818a43ed383e833a3a88ec768ef20ef75658bcbd9f7e846fab47f` |
| Exemplo_data.rvt | `c647c521262860bfa765fefda0d4c7542c9164a389d6af1c08b3d31d142588e2` |

```bash
cargo run --profile ci --example probe_re114_wall_lines_2023 -- modelo_bim.rvt
./target/ci/rvt-ifc modelo_bim.rvt -o modelo.ifc
python3 tools/re/element_materials_vs_ifc.py modelo.ifc modelo_bim.ifc
```

`rvt-gltf` exports both files.

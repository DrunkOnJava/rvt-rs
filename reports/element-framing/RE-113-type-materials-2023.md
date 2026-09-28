# RE-113 — Revit 2023 family instances' materials

**Date:** 2026-09-28
**Issues:** #421 (Revit 2023), #355 (materials); follows RE-58, RE-82, RE-111 and RE-112
**Artefacts (local only, no licence):** `255ribeiro/intro_ifc` `Exemplo_data.rvt` and `modelo_bim.rvt` (Revit 2023), each with Revit's own IFC4 export.
**Result:** Revit 2023 family types keep RE-82's geometry-material map, and 2023 materials are found by their class tag and named. Revit 2023 family instances are now written with the materials their type's geometry uses (`IfcMaterialConstituentSet`, RE-82), where before no 2023 element had a material.

| | Exemplo_data | modelo_bim |
|---|---:|---:|
| family instances with a material set | 18 | 27 |
| of them exactly Revit's | 17 (8 doors, 9 windows) | 27 (1 door, 10 windows, 8 beams, 8 columns) |
| with a set Revit's does not match | 1 | 0 |

Every 2024 and 2025 file is byte-identical: Core Interior, Snowdon Towers Architectural, RE1 Architecture, Projeto1 and teste_export_2025.

-----

## 1. The map

**The block.** A 2023 family type's parameter value block is `[owner u32][28 × ff][3 × 00]`, half of 2024's `[owner u64][56 × ff][3 × 00]` in both widths.

**The entries.** A map entry is `u32 key · u32 material`, 8 bytes where 2024's is 12. Example: door type 49486 `M_Folha única:0762 x 2134mm` holds `03 00 00 00 · 87 00 00 00 bd 28 00 00 · 88 00 00 00 f8 75 00 00 · 89 00 00 00 f8 75 00 00`, three entries naming materials 10429 and 30200. Its instances in Revit's export have the constituent set `Porta - Painel`, `Porta - Batente`.

**The first map only.** On 2024 the value block ends where the next one starts. A 2023 type's data runs on for kilobytes past its map, into single-entry lists naming other materials: `Padrão`, `Material pré-definido de parede`, `Telhado padrão`, `Vidro`, the same on every type. So only the block's first map is read (`partition_type_materials::type_material_names_2023`).

**System types.** A system-family type's data has its layer records where a family type's has its map, and a layer reads as a one-entry map: its function 1 as the count and its material as the value. The types of system-family elements are therefore left out.

**RE-84.** A block without a map is not read as a type that draws no geometry. On 2023 a missed map would otherwise turn a door into its opening alone.

## 2. The materials

**The class tag.** A 2023 material is an object `01 00 00 00 · u32 id` whose class tag `0x09fb` sits 0x27 bytes past the id, behind `00 00 00 · ff ff ff ff`. That is where 2024's `0x0a28` sits past a `u64` id (0x47), with the id and the `ff` run half as wide. The 28 bytes in between are usually `ff`, but not always: a family's own materials sometimes carry a `fc ff ff ff` word there. Found this way (`partition_materials::scan_materials_2023`):
- Exemplo_data has 103 materials: 64 open their own element data and 39 sit inside a family's.
- modelo_bim has 109.

**Names.** A material is named by its element data's first framed string (RE-111). A family's own material has no data of its own and is named instead by its first BuiltInParameter −1001203 entry: `i32 −1001203 · u32 0 · u32 n · UTF-16`, the 32-bit form of RE-58's `i64` entry. The name must be one: the entry's value is `U+FFFF` where unset.

**Matching Revit's names.** Of the material names in Revit's two exports, 9 of 11 and 9 of 10 are read. The rest are:
- `Aço, acabamento de pintura, marfim, fosco` on the roller door;
- `Revestimento – Branco`;
- `Concreto - Concreto moldado in loco` on modelo_bim's footings.

**Failing closed.** A type whose map names a material with no name read gets no materials, because a partial set would be a wrong one. That is why the roller door and the footings have none.

## 3. What differs

Door 337211 (`Porta-Passagem-Irregular-Nivelada`): its type's map also names `Alumín. 1`, which Revit's export leaves out of the door's set. This is the residual RE-82 records on 2024, a map that also covers a nested component.

## 4. End-to-end measurement

| file | sha256 |
|---|---|
| Exemplo_data.rvt | `c647c521262860bfa765fefda0d4c7542c9164a389d6af1c08b3d31d142588e2` |
| Exemplo_data.ifc | `42fdb8b0b540ebb993064e347ddf4a3e50d329f5cd8f45d0b6c73ee7a7da8c4a` |
| modelo_bim.rvt | `e06107f54481f408995ddf8f66093f1e4c4bc576ac0a3ff186edc4d32e21ff62` |
| modelo_bim.ifc | `58a6aa2872a818a43ed383e833a3a88ec768ef20ef75658bcbd9f7e846fab47f` |

```bash
cargo run --profile ci --example probe_re113_type_materials_2023 -- modelo_bim.rvt
./target/ci/rvt-ifc modelo_bim.rvt -o modelo.ifc
python3 tools/re/element_materials_vs_ifc.py modelo.ifc modelo_bim.ifc
```

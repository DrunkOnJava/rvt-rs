# RE-115 — Revit 2023 object styles give each category its material

**Date:** 2026-09-28
**Issues:** #421 (Revit 2023), #355 (materials); follows RE-91, RE-113 and RE-114
**Artefacts (local only, no licence):** `255ribeiro/intro_ifc` `Exemplo_data.rvt` and `modelo_bim.rvt` (Revit 2023), each with Revit's own IFC4 export.
**Result:** Revit 2023's object styles give each category a material, in a 2023 layout. A layer that takes its category's material now takes that one, as a 2024 wall layer does (RE-91), and now also on floors, roofs and ceilings.

Exemplo_data's walls and floor all take their category's material, so its elements with Revit's own material set go from 18 of 37 (RE-114) to 35 of 37 (`tools/re/element_materials_vs_ifc.py`):

| Exemplo_data | RE-114 | RE-115 | Revit's export |
|---|---|---|---|
| 17 walls | no material | `Material pré-definido de parede` | the same |
| floor | no material | `Material pré-definido de piso` | the same |

Every 2024 and 2025 file is byte-identical: Core Interior, Snowdon Towers Architectural, RE1 Architecture, Projeto1 and teste_export_2025. Their floors', roofs' and ceilings' layers name their own materials, or their category's entry is unset.

-----

## 1. The entry

On both 2023 projects every category has an entry, held twice (`partition_materials::CATEGORY_MATERIAL_FRAME_2023`):

```text
i32  BuiltInCategory
u32  ff ff ff ff
u32  1, then 2 in the second copy
u32  1
6    ff ff ff ff 3f 01
8    bytes not read
i32  -3000010
u32  material ElementId, ff × 4 when unset
```

Of Exemplo_data's 387 entries, 15 name a material, 8 hold a negative value that is no material, and the other 364 are unset. Walls, Floors and Roofs hold the materials Revit's own export writes for those categories' layers that take their category's material:

| category | material (RE-113 name) |
|---|---|
| Walls (−2000011) | 24 `Material pré-definido de parede` |
| Floors (−2000032) | 758 `Material pré-definido de piso` |
| Roofs (−2000035) | 25 `Telhado padrão` |
| Ceilings (−2000038) | unset |

2024's entry (RE-91) is `i64 category · ff × 8 · u32 1 · ff × 8 · u64 material`. RE-91's reader, `scan_category_material`, now reads the 2023 layout on 2023.

## 2. What is written

- **Walls.** RE-53's wall layers already took their category's material where it is set and named (RE-91). With RE-114's 2023 layers, Exemplo_data's walls now do: Revit writes each as an `IfcMaterial`, and rvt-rs as a one-layer set of the same material.
- **Floors, roofs and ceilings.** RE-57's stacked layers now take their category's material the same way, on every release.
- **modelo_bim's roof** stays without a material. Its record box is taller than its 0.4 m layer, as a pitched roof's is, and 2023 roof slopes are not read, so its layers are not attached (RE-57's height check).

## 3. End-to-end measurement

| file | sha256 |
|---|---|
| Exemplo_data.rvt | `c647c521262860bfa765fefda0d4c7542c9164a389d6af1c08b3d31d142588e2` |
| Exemplo_data.ifc | `42fdb8b0b540ebb993064e347ddf4a3e50d329f5cd8f45d0b6c73ee7a7da8c4a` |
| modelo_bim.rvt | `e06107f54481f408995ddf8f66093f1e4c4bc576ac0a3ff186edc4d32e21ff62` |

```bash
cargo run --profile ci --example probe_re115_category_materials_2023 -- Exemplo_data.rvt
./target/ci/rvt-ifc Exemplo_data.rvt -o exemplo.ifc
python3 tools/re/element_materials_vs_ifc.py exemplo.ifc Exemplo_data.ifc
```

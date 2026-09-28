# RE-119 — Revit 2023 beams sit on the Level at their top

**Date:** 2026-09-28
**Issues:** #421 (Revit 2023); follows RE-107 (2023 storeys) and RE-118 (building stories)
**Artefacts:** `255ribeiro/intro_ifc` `modelo_bim.rvt` and `Exemplo_data.rvt` (Revit 2023, local only, no licence), each with Revit's own IFC4 export; `Revit_IFC5_Einhoven.rvt` (Revit 2023, MIT, magnetar-io), with no export.
**Result:** positive, on the eight beams of one file. Revit's IFC export contains a 2023 structural framing element in the storey of the Level whose elevation equals the top of the element. Binding a beam to the one Level at its record box's top puts all 37 of modelo_bim's elements in Revit's storey (29 before, RE-107).

## 1. The observation

modelo_bim has four storeys (`Fundação` -1.7 m, `Nível 1` 0.0 m, `Nível 2` 4.0 m, `Nível 3` 8.0 m) and eight beams. Their vertical extents in Revit's export and their storeys:

| beams (ElementId) | bottom | top | RE-107 | Revit's storey |
|---|---:|---:|---|---|
| 337513, 337530, 337544, 337558 | 3.2 m | 4.0 m | `Nível 1` | `Nível 2` |
| 337632, 337633, 337634, 337635 | -0.8 m | 0.0 m | none (the building) | `Nível 1` |

In both groups Revit's storey is the Level at the beam's top. RE-107's Level for the upper beams is the Level their record names, which is not that one; the foundation beams' records name no Level RE-107 reads.

## 2. The rule

`partition_schema_mvp::resolve_framing_top_levels`, on Revit 2023 only: for an element of class `StructuralFraming` whose record box has `m_locationZ` and `m_bboxHeight`, the top is their sum. If exactly one Level's elevation is within 1e-3 ft of it, the element is bound to that Level (source `structural_framing_top_level`), replacing any Level it was bound to. No Level, or more than one, leaves the element as it was: the rule fails closed.

It is not applied to 2024 or 2025, where it is unmeasured.

## 3. End-to-end measurement

| | modelo_bim | Exemplo_data |
|---|---:|---:|
| elements in Revit's storey, RE-107 | 29 of 37 | 37 of 45 |
| elements in Revit's storey, RE-119 | 37 of 37 | 37 of 45 |

Exemplo_data has no beams; its eight others are nested trims rvt-rs leaves out, as before. Einhoven's output changes in nothing this rule touches. Every 2024 and 2025 file is byte-identical.

| file | sha256 |
|---|---|
| modelo_bim.rvt | `e06107f54481f408995ddf8f66093f1e4c4bc576ac0a3ff186edc4d32e21ff62` |
| modelo_bim.ifc | `58a6aa2872a818a43ed383e833a3a88ec768ef20ef75658bcbd9f7e846fab47f` |
| Exemplo_data.rvt | `c647c521262860bfa765fefda0d4c7542c9164a389d6af1c08b3d31d142588e2` |
| Exemplo_data.ifc | `42fdb8b0b540ebb993064e347ddf4a3e50d329f5cd8f45d0b6c73ee7a7da8c4a` |

```bash
./target/ci/rvt-ifc modelo_bim.rvt -o modelo.ifc
python3 tools/re/storeys_vs_ifc.py modelo.ifc modelo_bim.ifc
```

## 4. Open

- One file, eight beams. A beam whose top is at no Level (a sloped or offset beam) keeps its RE-107 binding; how Revit's export places one is unmeasured.
- Columns, braces and other framing-like categories are not covered.

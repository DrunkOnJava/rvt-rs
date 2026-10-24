# RE-206: a category's material from its graphics styles; nested families' parameter-driven visibility

**Date:** 2026-10-21
**Issue:** #355; follows RE-91, RE-115, RE-204
**Artefacts:** Snowdon Towers Sample Architectural (Revit 2025 copy, sha256 `8ac90a85…d334`) against Revit's IFC4 export (`bldrs-ai/test-models`, sha256 `ecfcb04e…985a`), measured locally only: it cannot be fetched in CI. Checked for regressions on `2024_Core_Interior.rvt` against `2024_Core_Interior_slim.ifc`, the four RE1 models and the 22 R2025 models (`R2025_Models.zip`, #356).
**Probes:** `examples/probe_re206_category_styles.rs`, `examples/probe_re204_family_forms.rs`.
**Status:** positive. Exported.

## 1. The question

Two Snowdon elements still differed: wall 619550 lacked "Default Wall", and AC_Bandstand Canopy had "Acrylic, Clear" where Revit writes only `<Unnamed>`. RE-91 had noted that Snowdon's object-styles Walls entry holds 0 while Revit still writes "Default Wall" for its walls whose layer takes its category's material, "an unset entry falls back to something not read". RE-91 and RE-115 are not read on Revit 2025 at all.

## 2. Result

### 2.1 Category materials

- Wall type 1533243's exterior layer has `m_materialId` −1, its category's material.
- The category's `GStyleElem` objects (`m_categoryId` the BuiltInCategory) each hold a `GStyle` whose `m_materialElemId` names it: on Snowdon Walls → "Default Wall", Floors → "Default Floor", Roofs → "Default Roof". `scan_category_material` falls back to it where the object-styles entry is unset or not read.
- Snowdon: wall 619550 wrong → Revit's, and 15 walls none → "Default Wall".
- R2025 models: every wall gets Revit's "Default Wall", 117 walls in 21 models, none before. `tests/element_materials.rs::r2025_walls_take_their_category_material` fails with the fallback disabled (117 of 117 differ) and passes with it.

### 2.2 Nested families

- The canopy's geometry is all nested families. Its type's map holds "Acrylic, Clear" from a nested dome family, whose one revolution's `Visible` is bound (`FamilyParametrizedElemParamsCell`, `m_elemPropId` −1006205) to its family parameter "Include Dome".
- The nested type sets "Include Dome" to 1 and the canopy's one instance of it to 0. Parameters are matched across the two documents by caption (`ParamDef*.m_caption`): their ids differ.
- `native_family_forms` now reads nested families' documents and evaluates such bindings on the nested instance, else its type. A type left with only geometry with no material is `<Unnamed>`.

| | before | after |
|---|---:|---:|
| Snowdon Towers, sets equal to Revit's | 5,117 | 5,134 of 5,134 given |
| R2025 models 01 to 18, walls with Revit's set | 0 | 117 |

None goes from Revit's set to another; Core Interior and RE1 are unchanged.

## 3. Cost

The family documents and native names add passes; with the names and category styles memoised on `RevitFile` and the family walks reusing the inflated partitions, Snowdon's export takes 23.3 s against 19.8 s on `main`.

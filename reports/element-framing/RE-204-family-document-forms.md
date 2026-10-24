# RE-204: `<Unnamed>` only where a family shows geometry with no material

**Date:** 2026-10-21
**Issue:** #355; follows RE-203, RE-163
**Artefacts:** Snowdon Towers Sample Architectural (Revit 2025 copy, sha256 `8ac90a85…d334`) against Revit's IFC4 export (`bldrs-ai/test-models`, sha256 `ecfcb04e…985a`), measured locally only: it cannot be fetched in CI. Checked for regressions on `2024_Core_Interior.rvt` against `2024_Core_Interior_slim.ifc`, the four RE1 models and the 22 R2025 models (`R2025_Models.zip`, #356).
**Probe:** `examples/probe_re204_family_forms.rs`.
**Status:** positive. Exported. Extended by RE-205 and RE-206.

## 1. The question

After RE-203, 23 elements whose type maps some geometry to −1 got `<Unnamed>` where Revit's export writes none: 21 Washer_Dryer_Combo and 1 TableWithUmbrella on Snowdon Towers, and RE1 Electrical's 284621-FirePanel. The same rule is right on 31 Sink Vanity-Round, 21 refrigerators and others. What tells them apart?

## 2. Method

The type records hold nothing that does, and the saved faces of every type have `m_renderStyleId` −1. A loaded family's own document is a keyed block of the project's partitions, named by the key its `Family` record holds (RE-163); it decodes with the project's schema, and its element ids are in the project's id space. Its forms (`ExtrusionElem`, `RevolutionElem`, `SweepElem`, `BlendElem`, `SweptBlendElem`, `FreeFormElement`, `Text3dElem`) hold `m_materialId`, `m_cutting` and `Visible` (`IS_VISIBLE_PARAM`, −1006205) in their `ParamValueSetInt`. Scored with `tools/re/element_materials_vs_ifc.py` (IfcOpenShell 0.9.0.post1), element by element against the build before the change.

## 3. Result

| family | forms with no material, not void | their Visible | Revit writes `<Unnamed>` |
|---|---|---|---|
| Washer_Dryer_Combo | 4 extrusions | 0 | no |
| TableWithUmbrella | 2 sweeps | 0 | no |
| Sink Vanity-Round | 1 extrusion | 1 | yes |
| 284621-FirePanel (RE1 Electrical) | none: 93 free-form solids and model text, all with a material; nests no family | | no |
| Cash Register_Touch Screen_Generic | none; nests a family instance | | yes |

The rule: a type's −1 geometry is `<Unnamed>` unless every form with no material is hidden or void, or there is none and the family nests no instance and every form decodes.

| | before | after |
|---|---:|---:|
| Snowdon Towers, sets equal to Revit's | 5,092 | 5,114 |
| RE1 Electrical | 48 | 49 of 49 |

None goes from Revit's set to another; the other exports are unchanged.

## 4. What this changes

- `native_family_forms` reads a family's document; `native_document::group_records` is `pub(crate)` for it.
- `refine_type_materials` adds `<Unnamed>` only for types whose family shows such geometry.

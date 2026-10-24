# RE-201: a mullion or curtain panel type's material is its `m_renderStyleId`

**Date:** 2026-10-21
**Issue:** #355; follows RE-166 (B66)
**Artefacts:** Snowdon Towers Sample Architectural (Revit 2025 copy, sha256 `8ac90a85…d334`) against Revit's IFC4 export (`bldrs-ai/test-models`, sha256 `ecfcb04e…985a`), measured locally only: it cannot be fetched in CI. Checked for regressions on `2024_Core_Interior.rvt` against `2024_Core_Interior_slim.ifc`, the four RE1 models and the 22 R2025 models (`R2025_Models.zip`, #356).
**Probe:** `examples/probe_re201_curtain_render_styles.rs`.
**Status:** positive. Exported.

## 1. The question

RE-166 reads a mullion type's material at +135 of its data object (+139 for a panel type). On Snowdon Towers 1,065 of the 1,425 mullions Revit's export gives a material got none from rvt-rs (847) or `<Unnamed>` (218), and 98 of 348 panels none. Where is their material?

## 2. Method

The probe reads every `SysMullionFamSym` and `SysPanelFamSym` record with `native_document::extract_graphs` and prints its `m_renderStyleId` and that material's name. Each type's material was compared with the material Revit's export relates to the `IfcMember` and `IfcPlate` elements of that type.

## 3. Result

- `m_renderStyleId` is a `MaterialElem` on 492 of Snowdon's 497 mullion and panel types (the probe's last line); the other five are the "Empty" panel type and its copies, −1, which Revit exports without a material. On Snowdon Towers it is Revit's material for all 17 exported types: 13 mullion types (1,425 members) and 4 panel types (348 plates), 1,773 of 1,773. Per-instance copies of a type (`m_masterId` set) hold their master's value.
- On RE1 Architecture it is "Aluminum 2" (mullions) and "Glass" (panels), as RE-166's offsets read.
- The offsets fail because a field near +44 is 6 bytes on some types and 4 on others: on 10 of Snowdon's 13 exported mullion types the material sits at +133. It always follows `ff ff ff ff 3c 10`.

| Snowdon Towers | before | after |
|---|---:|---:|
| mullions with Revit's set | 360 | 1,425 |
| panels with Revit's set | 250 | 348 |
| element sets equal to Revit's | 3,689 | 4,852 |

Element by element: 847 mullions none → Revit's, 218 wrong → Revit's, 98 panels none → Revit's, none the other way. Every other export measured is byte-identical.

## 4. What this changes

- `partition_curtain_materials::scan_curtain_type_materials` reads `m_renderStyleId` by name first; the offsets fill only the types it does not read.
- `attach_curtain_materials` replaces an `<Unnamed>`-only set (#580) with the type's material.

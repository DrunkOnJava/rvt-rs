# RE-205: the last off-centre columns; a material only hidden forms have is not exported

**Date:** 2026-10-21
**Issue:** #355; completes RE-202, extends RE-204
**Artefacts:** Snowdon Towers Sample Architectural (Revit 2025 copy, sha256 `8ac90a85…d334`) against Revit's IFC4 export (`bldrs-ai/test-models`, sha256 `ecfcb04e…985a`), measured locally only: it cannot be fetched in CI. Checked for regressions on `2024_Core_Interior.rvt` against `2024_Core_Interior_slim.ifc`, the four RE1 models and the 22 R2025 models (`R2025_Models.zip`, #356).
**Probes:** `examples/probe_re202_column_joins.rs`, `examples/probe_re204_family_forms.rs`.
**Status:** positive. Exported.

## 1. The question

After RE-202 two of Snowdon's 108 off-centre columns kept another layer than Revit's: 1918459, whose first join is the foundation wall 679356 below it, and 1366814, a 2 ft column centred 0.04 ft inside a 1.05 ft exterior wall. And Refrig_Base_wDrawers kept "Clearance", which Revit's export leaves out.

## 2. Result

- **The wall:** the first join among the walls the decode lists as overlapping the column (`m_column_joined_wall`), and the first join where it lists none. 1918459 then takes wall 676281's interior layer, Revit's "Gypsum Wall Board".
- **Standing out past both faces:** a column whose box reaches past both faces of its wall takes the wall's exterior layer. 51 columns do; 50 are in walls with "Gypsum Wall Board" on both faces, and 1366814 takes "Metal Panels 50"x60" - White", Revit's. The rule rests on that one asymmetric case.
- **Hidden forms:** Refrig_Base_wDrawers's family has one extrusion in "Clearance", with `Visible` 0 (bound to a family parameter). A material that only hidden forms of a type's family have is dropped from its set.

| Snowdon Towers | before | after |
|---|---:|---:|
| off-centre columns with Revit's set | 106 | 108 |
| element sets equal to Revit's | 5,114 | 5,117 |

None goes from Revit's set to another; the other exports are unchanged.

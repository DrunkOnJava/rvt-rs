# RE-202: an off-centre column takes a layer of the wall it is joined to; rendered content is `<Unnamed>`

**Date:** 2026-10-21
**Issue:** #355; follows #580 and RE-149
**Artefacts:** Snowdon Towers Sample Architectural (Revit 2025 copy, sha256 `8ac90a85…d334`) against Revit's IFC4 export (`bldrs-ai/test-models`, sha256 `ecfcb04e…985a`), measured locally only: it cannot be fetched in CI. Checked for regressions on `2024_Core_Interior.rvt` against `2024_Core_Interior_slim.ifc`, the four RE1 models and the 22 R2025 models (`R2025_Models.zip`, #356).
**Probe:** `examples/probe_re202_column_joins.rs`.
**Status:** positive. Exported. Completed by RE-205.

## 1. The question

#580 gives a column whose type has no material the joined walls' materials where they all name one set, else `<Unnamed>`. On Snowdon Towers Revit's export gives each of the 108 "Rectangular Column (Off Center)" one material, mostly "Gypsum Wall Board"; rvt-rs gave 94 of them `<Unnamed>` (81) or every layer of their walls (13). Separately, 17 "Tree - Honey-Locust" got their map's two materials where Revit writes `<Unnamed>`.

## 2. Method

The column's own record and its per-instance symbol hold no material. Its joins are its `CutPairGStep` objects, `m_otherId` the joined element, made in `m_id` order. A wall's record names its location curve in `VWallDriver.m_pCrv` (a `GLine`) and whether it is flipped in `SWall.m_isFlipped`; `VWallDriver.m_flip` is false on all 198 joined walls. Layers were read exterior first. Scored with `tools/re/element_materials_vs_ifc.py` (IfcOpenShell 0.9.0.post1), element by element against the build before the change.

## 3. Result

- Revit gives the column the layer of its first joined wall on the side the column stands: the exterior layer if the column's box centre is on the exterior side, else the interior one. The exterior lies left of the location line unless the wall is flipped (16 of 19 exterior walls face away from the building that way).
- The first join in `CutPairGStep` order, not the decoded `m_column_joined_wall` list, which is sorted by id and holds only walls overlapping the column's box.
- A layer whose material the decode does not name is not taken (wall 619550's exterior layer; RE-206 names it).
- 106 of 108 at this step. The two left: 1366814 straddles its wall; 1918459's first join is a foundation wall below it (RE-205).
- Rendered content: `Family.m_eRenderModelType` 4 (`native_rendered_content`). Revit writes one material, `<Unnamed>`, for every instance: 17 Honey-Locust, 100 RPC Shrub, 6 RPC Female, 4 RPC Male, 127 of 127. No family of Core Interior or RE1 has the value.

| Snowdon Towers | before | after |
|---|---:|---:|
| element sets equal to Revit's | 4,852 | 4,961 |

92 columns and 17 trees wrong → Revit's, none the other way; the other exports are unchanged.

## 4. What this changes

- `native_column_joins`: a column's joins in order, and a straight wall's side.
- `native_rendered_content`: the types whose family is rendered content.
- `partition_schema_mvp`: `attach_first_join_materials` before #580's joined-walls step; `attach_rendered_content_materials`.

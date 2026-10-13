# RE-178 — A column's IFC body stops where a joined slab or roof cuts it

**Date:** 2026-10-13
**Artefacts:** Autodesk's public Revit 2024 samples (`racadvancedsampleproject`, `racbasicsampleproject`, `rmeadvancedsampleproject`, `rmebasicsampleproject`, `rstadvancedsampleproject`, `rstbasicsampleproject`, `Technicalschoolcurrentm`) with Revit's own IFC4 export, and the 12 BIM4LCA / Nordic LCA (CC BY-SA 4.0) `ARCH` Revit files with Revit's own IFC4 exports.
**Clean-room provenance:** public Autodesk Revit 2024 sample files, public BIM4LCA files, Revit's own IFC4 export as the oracle, and the file's own schema and exported symbol names only. No SDK source and no decompiled or disassembled code.
**Result:**
- **The problem.** Revit's IFC export cuts a column's body where a floor or roof joined to it cuts it: the removed span is the column/slab intersection, always at a slab or roof face, and the cut body is still a plain vertical extrusion. rvt-rs drew every column to its own extent, so 276 columns on these files ran through the slab above (or, once, started inside the slab below).
- **Where the cut is stored.** The column's geometry node (RE-173: `u64 id · u64 3 · u64 -1`, two boxes of six `f64` at 96 and 48 bytes before the id) holds the column's own extent in the first box and its extent after joins in the second.
- **The rule, fail closed.** For every `Column` and `StructuralColumn` (Revit 2024), take the second box's z-range only when every node of the column, in every partition, has x and y extents equal to the column's (both boxes, 1e-6 ft), all of them give the same second-box z-range (0.01 ft), and that range lies inside the column's own extent and differs from it. The element gets `m_column_body_bottom` / `m_column_body_top`; `m_locationZ`, `m_bboxHeight` and every parameter keep the column's own extent. A column whose copies disagree, or which has no node, keeps its body.
- **The body.** The extrusion depth becomes the cut height. The placement stays at the column's own base, so storeys and containment do not move; a body that starts above the base is written as a placed extrusion that far up. `BoundingBoxHeight` stays the own extent and the element carries `BodyHeightSource = partition_geometry_node_after_joins`. No boolean.

Measured on this branch with `rvt-ifc` against Revit's IFC body z-range (vertical `IfcExtrudedAreaSolid`, 0.01 ft at both ends), before = 4a70c53:

| file | columns compared | right before | right after | fixed | regressed |
|---|---:|---:|---:|---:|---:|
| rstadvanced | 203 | 14 | 203 | 189 | 0 |
| rstbasic | 30 | 29 | 30 | 1 | 0 |
| racadvanced / Technicalschool | 176 / 176 | all | all | 0 | 0 |
| racbasic | 2 (+1 faceted) | 2 | 2 | 0 | 0 |
| rmeadvanced / rmebasic | 135 / 135 | all | all | 0 | 0 |
| Office_Concrete_As-Built | 279 | 265 | 279 | 14 | 0 |
| Office_Concrete_BuildingPermit | 327 (+18 not extrusions) | 262 | 327 | 65 | 0 |
| Office_Timber_As-Built | 339 (+5) | 337 | 339 | 2 | 0 |
| Office_Timber_BuildingPermit | 335 (+5) | 330 | 331 | 1 | 0 |
| Housing x6 (Concrete, Timber, Terrain) | 7, 7, 5, 5, 54, 54 | all | all | 0 | 0 |
| Office_Terrain x2 | 0 columns | | | | |

272 of 276 cut columns fixed, none regressed (2,269 columns compared, 1,993 right before, 2,265 after); RE-173's columns (rmeadvanced 131, Housing) stay right. The 4 left are in `Office_Timber_BuildingPermit`, among the 14 columns whose node copies give different second-box z-ranges (partition 374 holds older extents); the rule leaves them as they were. Across every file, storeys, product placements, containment and all other properties are unchanged between the two exports. `DACHsampleproject` and Snowdon do not export on this base (#736).

**Limits.** Plan-shape cuts are not handled (the node box is a box). Which partition's copy is current is not decoded, so columns with disagreeing copies are left alone. Revit 2023 and 2025 are not checked.

Check: `tests/column_cut_body.rs` (`RVT_NORDIC_LCA_DIR` with `ARK_NordicLCA_Office_Concrete_As-Built.rvt`: 14 columns' bodies equal Revit's, 1.476 ft below their own top, which `BoundingBoxHeight` keeps; on 4a70c53 they run to their own top and it fails. Optional: `RVT_REVIT2024_SAMPLES_DIR` with `rstbasicsampleproject.rvt`, column 1173925 cut at its bottom).

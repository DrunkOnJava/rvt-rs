# RE-173 — A column's z-range from its instance geometry node

**Date:** 2026-10-13
**Artefacts:** Autodesk's public Revit 2024 samples (`racadvancedsampleproject`, `racbasicsampleproject`, `rmeadvancedsampleproject`, `rstadvancedsampleproject`) with Revit's own IFC4 export, and BIM4LCA / Nordic LCA (CC BY-SA 4.0) `Housing_Concrete_BuildingPermit`, `Housing_Timber_As-Built`, `Office_Concrete_BuildingPermit`, `Office_Timber_BuildingPermit`.
**Clean-room provenance:** public Autodesk Revit 2024 sample files, public BIM4LCA files, Revit's own IFC4 export as the oracle, and the file's own schema and exported symbol names only. No SDK source and no decompiled or disassembled code.
**Result:**
- **The problem.** A column's element record box (what rvt-rs reads into `m_locationZ` and `m_bboxHeight`) can start at exactly z = 0.0 while the column starts higher. Main then draws the column from the ground.
- **The node.** In the inflated `Partitions/*` streams the column's `u64` ElementId occurs again in its geometry node: `u64 id · u64 3 · u64 0xffffffffffffffff`, with two boxes of six `f64` (min x, y, z, max x, y, z) at 96 and 48 bytes before the id. Their x and y extents are equal and the second holds Revit's z-range. The tags around them are the schema classes `GArray` (`0x0820`, parent `GInstance`) and `GRep` (`0x0841`). A column can have several such nodes and they do not always agree.
- **The rule, fail closed.** Only for a `Column` or `StructuralColumn` whose record box starts at exactly 0.0. A node counts when both boxes are finite with max above min, their x and y extents are equal, those extents equal the record's to 1e-6 ft, its top is the record's top (0.01 ft) and its bottom is not the record's. The bottom moves only if every counting node gives the same one; the top stays. The element carries `m_column_z_source = partition_geometry_node`. Revit 2024 only.
- **Cost.** One `memchr::memmem` pass per partition over the 16 bytes after the id, only when a column starts at 0.0.

Measured on this branch, per column against Revit (Base Level plus Base Offset and Top Level plus Top Offset for Autodesk files; Revit's IFC geometry for Nordic):

| file | columns | rewritten | rewritten and equal to Revit | not rewritten, equal to Revit before and after |
|---|---:|---:|---:|---:|
| rmeadvanced | 135 | 131 | 131 | 4 |
| Housing_Concrete_BuildingPermit | 7 | 7 | 7 | 0 |
| Housing_Timber_As-Built | 54 | 54 | 54 | 0 |
| racadvanced | 176 | 0 | | 164 right, 12 wrong either way |
| rstadvanced | 203 | 0 | | 203 |
| racbasic | 3 | 0 | | 2 right, 1 wrong either way |
| Office_Concrete_BuildingPermit | 346 | 0 | | unchanged |
| Office_Timber_BuildingPermit | 340 | 0 | | unchanged |

On main the 192 rewritten columns start at 0.0 (rmeadvanced 0.309, 12.467 and 23.950 ft are Revit's). No column main gets right changes.

**Limits.** Columns whose record box does not start at 0.0 are never touched, even if wrong. A column with no counting node, or nodes that disagree, is left as it is. Revit 2023 and 2025 are not checked. The levels of `Housing_Timber_*` are a separate matter (their shared elevation); the rule needs no Levels.

Check: `tests/column_node_z.rs` (`RVT_NORDIC_LCA_DIR` with `ARK_NordicLCA_Housing_Concrete_BuildingPermit.rvt`: the 7 columns' bottoms and tops equal Revit's; on main the bottoms are 0.0 and it fails. Optional: `RVT_REVIT2024_SAMPLES_DIR` with `rmeadvancedsampleproject.rvt`, 131 columns above z = 0).

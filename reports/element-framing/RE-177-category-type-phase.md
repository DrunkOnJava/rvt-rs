# RE-177 — Category, family/type and Phase Created in Revit's property sets

**Date:** 2026-10-13
**Follows:** RE-38 (family/type), RE-168 (the reference list's leading slot is the phase).
**Artefacts:** Autodesk's public Revit 2024 samples, each with Revit's own IFC4 export as the oracle.
**Clean-room provenance:** public sample files and Revit's own IFC export only; the class name `ProjectPhase` comes from the file's own schema stream. No SDK source and no decompiled or disassembled code.
**Result:**
- **Family and type were already exported** (`ObjectType`, the `IfcTypeObject` name, the type object's `Tag` = type id) and agree with Revit's on every element that has them. Revit writes them under other names: `Family`, `Family and Type`, `Type`, `Type Id` in `Other` on the occurrence, `Family Name` (`Other`) and `Type Name` (`Identity Data`) on the type. Those names are now written too.
- **Category** is the element record's BuiltInCategory at `+0x12` (`m_builtinCategory`). The file stores the id only. On the seven samples that export elements on main, 57 ids occur across 26,656 elements and no id appears with two names in Revit's export, so `builtin_category_name` maps those ids to the English names Revit's export writes (sub-categories as `Parent: Name`). An id not in the table writes nothing.
- **Phase Created** is the one phase id in the element record's reference list (`+0x88`, RE-168). A phase's name follows its ElementId in the frame `01 00 00 00 | u64 id | 56 x ff | 00 00 00 | u32 n | UTF-16 x n`. The same frame names levels, grids and views, so a hit counts as a phase only when the `ProjectPhase` class tag is the `u16` 38 bytes before the id, or the frame is the element's own data and the id has an `OST_Phases` (-2000112) record. None or two phase ids writes nothing.

## Phase rule on nine samples (Python probe, before implementation)

| file | right | wrong | no phase | phases |
|---|---:|---:|---:|---|
| rstadvanced | 936 | 0 | 0 | New Construction (0), Existing (2) |
| rstbasic | 816 | 0 | 0 | |
| racadvanced | 5408 | 0 | 4 | |
| racbasic | 430 | 0 | 0 | Working Drawings |
| rmeadvanced | 6688 | 0 | 1 | |
| rmebasic | 6702 | 0 | 1 | |
| Technicalschool | 5403 | 0 | 4 | |
| DACH | 2606 | 0 | 147 | Phase 1 (20), Phase 2 (2586) |
| Snowdon Architectural | 5746 | 0 | 1 | |

DACH is the discriminating file: two phases in use, never confused. Elsewhere one phase is in use, so the rule is a membership check there. No sample demolishes anything, so `Phase Demolished` is not covered and whether the slot is Phase Created or the phase shown is not told apart beyond Revit's `Phase Created` agreeing.

## rvt-rs export against Revit (equal / elements where Revit has the property; no wrong values)

| file | Category | Phase Created | Family | Type | Type Id |
|---|---:|---:|---:|---:|---:|
| Technicalschool | 5403 / 5403 | 5401 / 5401 | 5354 / 5393 | 5354 / 5393 | 5354 / 5393 |
| racadvanced | 5408 / 5408 | 5406 / 5406 | 5359 / 5398 | 5359 / 5398 | 5359 / 5398 |
| racbasic | 424 / 427 | 424 / 424 | 411 / 424 | 413 / 424 | 413 / 424 |
| rmeadvanced | 6483 / 6492 | 6477 / 6483 | 5223 / 6483 | 5223 / 6483 | 5223 / 6483 |
| rmebasic | 6527 / 6536 | 6521 / 6527 | 5268 / 6527 | 5268 / 6527 | 5268 / 6527 |
| rstadvanced | 904 / 904 | 904 / 904 | 904 / 904 | 904 / 904 | 904 / 904 |
| rstbasic | 454 / 454 | 454 / 454 | 450 / 454 | 450 / 454 | 450 / 454 |

Type objects, `Family Name` and `Type Name`: 22,948 of 25,471, none wrong. The family/type gap is mostly ducts and pipes (about 1,250 per rme file), unchanged from main.

**DACH and Snowdon** export no elements on main (#736). With #737 applied locally (not part of this change): DACH `Category` 2381 / 2395 and `Phase Created` 2370 / 2379, Snowdon 5657 / 5708 and 5666 / 5685, none wrong; Snowdon family/type none wrong; DACH family/type 14 differ from Revit (12 stair runs typed `陇`, 2 walls with another wall's type), from the RE-38 decode, which this change does not touch.

**Test:** `tests/category_type_phase.rs` (`RVT_REVIT2024_SAMPLES_DIR`): every value written equals Revit's, and `Category` and `Phase Created` cover at least 99%. Fails on main (`Category 0 of 6609`), passes with the change.

**Limits.** Revit 2024 only. English category names, 57 ids. `Phase Demolished` not covered.

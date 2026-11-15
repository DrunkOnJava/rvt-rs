# RE-487: before Revit 2024, whether the type map's total geometry tags or the maps of the types a family's nested instances draw say which families write `<Unnamed>` (run pending)

**Date:** 2026-11-15
**Issues:** #1603 (Revit 2015 to 2023: the 70 windows of the spent Revit 2018 project lack Revit's materials); follows RE-485 (`<Unnamed>` is all-or-none within every family before 2024: 3 of the spent project's 7 window families write it on all their windows and 4 on none), RE-483, RE-482, RE-481.
**Probe and scorer:** `probe_1603_unnamed_nested_types_before_2024` and `tools/re/window_door_unnamed_nested_types_vs_ifc.py`. No `src/` change.
**Status:** pending the run. Rows registered on #1603 (comment 6894610678) before it.

## 1. The question

RE-485 found the family decides `<Unnamed>` before 2024, and that the type map's total geometry tags follow the spent project's families (33 tags: 8 of 8 written; 21: 44 of 45; 13: 0 of 15) except one window. Two reads are left that are cheap: the total on the other pre-2024 projects, where any family with 21 or more tags and none written drops the lead; and the maps of the types the family's nested instances draw, which every window of the spent project and of BIM Whale nests.

## 2. What the probe and the scorer count

For each door and window the export writes, the probe prints the drawn symbol's and the family's ElementIds, the number of geometry tags the symbol's `m_geomTag2MaterialId` holds and how many map to no material, and of the family's nested family instances how many it has, how many draw a type of the family's document whose map was read and how many of those maps hold a geometry tag mapped to no material. The scorer counts, per project and IFC class, for the elements whose type map holds geometry mapped to no material and to which Revit gives materials, `<Unnamed>` written or not by the symbol's total tags, and by whether the family has no nested instance, nested types read with none holding an unset tag, a nested type holding one, or a nested type not read. Each record of a family's document is read once, from its last copy. Counts only for held-out projects.

## 3. The registered rows

Registered on #1603 before the run (2026-11-15, comment 6894610678).

1. **Total tags, every project.** The elements by the type map's total geometry tags, written and not written, per project and class. Reported; any other pre-2024 family with 21 or more tags and none written drops the lead.
2. **The nested instances' own types.** The elements by whether any nested type's map holds a -1, written and not written, per project. Reported.
3. **A rule** only if right on every element of every pre-2024 project; otherwise nothing is written, and with this run the question is closed for Revit 2015 to 2023 unless a read finds a family fact that separates all 16 families.
4. **Nothing moves.** No `src/` change.

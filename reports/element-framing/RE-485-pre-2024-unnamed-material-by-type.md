# RE-485: before Revit 2024, whether `<Unnamed>` in a door or window's material set is constant within a drawn symbol and a family (run pending)

**Date:** 2026-11-15
**Issues:** #1603 (Revit 2015 to 2023: the 70 windows of the spent Revit 2018 project lack Revit's materials); follows RE-483 (a form's bound `Visible` read from its parameter does not decide `<Unnamed>`: 15 of the spent project's 68 windows share every fact read and 8 have it written), RE-482, RE-481.
**Probe and scorer:** `probe_1603_unnamed_by_type_before_2024` and `tools/re/window_door_unnamed_by_type_vs_ifc.py`. No `src/` change.
**Status:** pending the run. Rows registered on #1603 (comment 6894163538) before it.

## 1. The question

RE-483 left 15 of the spent project's 68 windows with identical facts of the host family's forms and `<Unnamed>` written on 8 of them. Whether the cause is the type's or the instance's is not in those counts. Is `<Unnamed>` constant within a drawn symbol (Revit's type) and within a family, and what do the symbol's map tag counts say?

## 2. What the probe and the scorer count

For each door and window the export writes, the probe prints RE-482's facts of the family's forms and the drawn symbol's and the family's ElementIds, the number of geometry tags the symbol's `m_geomTag2MaterialId` holds and how many map to no material. The scorer counts, per project and IFC class, for the elements whose type map holds geometry mapped to no material and to which Revit gives materials: the drawn symbols and families whose elements all have `<Unnamed>` written, none of them, or some; the fact groups (elements sharing every fact of RE-482's probe) with both outcomes and whether each is constant within a symbol, a family or an unset tag count; and the elements by the number of unset tags (1, 2, 3 or more). Counts only for held-out projects.

## 3. The registered rows

Registered on #1603 before the run (2026-11-15, comment 6894163538).

1. **Type.** Reported per project: drawn symbols with `<Unnamed>` on all, none, and some of their elements.
2. **Family.** The same for families.
3. **The 15.** The spent project's 15 windows with identical host-form facts: their symbols and families and the written count of each.
4. **Unset tags.** The elements by the number of unset tags, written and not written, per project; a rule is worth proposing only if it is right on every element of every pre-2024 project.
5. **Nothing moves.** No `src/` change.

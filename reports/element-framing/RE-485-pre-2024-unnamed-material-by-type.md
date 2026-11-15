# RE-485: before Revit 2024 `<Unnamed>` in a door or window's material set is constant within a family on every project: the spent Revit 2018 project's 7 window families write it on all of their windows (3 families, 52 windows) or on none (4 families, 16), and no fact read says which

**Date:** 2026-11-15
**Issues:** #1603 (Revit 2015 to 2023: the 70 windows of the spent Revit 2018 project lack Revit's materials); follows RE-483 (a form's bound `Visible` read from its parameter does not decide `<Unnamed>`: 15 of the spent project's 68 windows share every fact read and 8 have it written), RE-482, RE-481.
**Artefacts:** the spent Revit 2018 project (the held-out save of it carries the same lines and is counted once), Frigento (Revit 2020), Warmond (Revit 2021) and BIM Whale (Revit 2021), 431 doors and windows whose type map holds geometry mapped to no material and to which Revit gives materials. The spent project's and BIM Whale's references are IFC2X3 exports and Warmond's is an IFC4 export passed through BIMServer.center's preprocessor, all counted apart and never removing a value; Frigento's 22 doors are the only ones measured against an unmodified Revit IFC4 export.
**Probe and scorer:** `probe_1603_unnamed_by_type_before_2024` and `tools/re/window_door_unnamed_by_type_vs_ifc.py` at `fa38cbf51a8c` (Measure [43714791546](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43714791546)), archived under `refs/archive/2026-11-15/cited/`. No `src/` change.
**Status:** positive for the unit, negative for a rule. On every pre-2024 project `<Unnamed>` is all-or-none within a drawn symbol and within a family (none of 119 symbols and none of 16 families is mixed). On the spent project's 68 windows it is a property of the 7 families: written on every window of 3 of them (36, 8 and 8 windows) and on none of 4 (7, 4, 4 and 1); it is not constant within an unset tag count, and no count of unset tags, no state of the nested instances and none of RE-483's facts separates the families. The 15 windows RE-483 found with identical facts are two families: 8 windows of one write it and 7 of another do not. Why one family writes it is not read.

## 1. The question

RE-483 left 15 of the spent project's 68 windows with identical facts of the host family's forms and `<Unnamed>` written on 8 of them. Whether the cause is the type's or the instance's is not in those counts. Is `<Unnamed>` constant within a drawn symbol (Revit's type) and within a family, and what do the symbol's map tag counts say?

## 2. What the probe and the scorer count

For each door and window the export writes, the probe prints RE-483's facts of the family's forms (effective visibility), the nested instances' stored and bound `Visible` and the drawn symbol's and the family's ElementIds, the number of geometry tags the symbol's `m_geomTag2MaterialId` holds and how many map to no material. The scorer counts, per project and IFC class, for the elements whose type map holds geometry mapped to no material and to which Revit gives materials: the drawn symbols and families whose elements all have `<Unnamed>` written, none of them, or some; the fact groups (elements sharing every fact of RE-483's probe) with both outcomes and whether each is constant within a symbol, a family or an unset tag count; the elements by the number of unset tags (1, 2, 3 or more), and by the visibility of the family's nested instances (their own stored `Visible` and whether it is bound to a parameter). Each record of a family's document is read once, from its last copy. Counts only for held-out projects. The lines are printed for every release; this report reads those of Revit 2015 to 2023 (a Revit 2026 held-out house in the run, 13 doors and 12 windows with `<Unnamed>` on all 25, is RE-481's).

## 3. Measured

Measure 43714791546, 84 models, the spent project listed per element.

**Rows 1 and 2: symbol and family.** Per project and IFC class, the drawn symbols and the families whose elements have `<Unnamed>` written on all of them, on none and on some:

| Project | Class | Elements | Symbols: all / none / mixed | Families: all / none / mixed |
| --- | --- | ---: | --- | --- |
| Spent Revit 2018 (IFC2X3) | windows | 68 | 43 / 13 / 0 of 56 | 3 / 4 / 0 of 7 |
| Frigento (Revit 2020) | doors | 22 | 0 / 13 / 0 of 13 | 0 / 2 / 0 of 2 |
| Warmond (Revit 2021, preprocessed, counted apart) | windows | 54 | 0 / 10 / 0 of 10 | 0 / 3 / 0 of 3 |
| Warmond (counted apart) | doors | 14 | 0 / 2 / 0 of 2 | 0 / 2 / 0 of 2 |
| BIM Whale (Revit 2021, IFC2X3, counted apart) | windows | 263 | 0 / 35 / 0 of 35 | 0 / 1 / 0 of 1 |
| BIM Whale (counted apart) | doors | 10 | 0 / 3 / 0 of 3 | 0 / 1 / 0 of 1 |

The 43 written symbols of the spent project hold its 52 windows with `<Unnamed>` and the 13 others its 16 without. Only the spent project has a family that writes it. Most symbols hold one window (56 symbols for 68 windows), so the symbol column says little by itself; the family column is the one that groups.

**Row 3: the 15.** The 15 windows of the spent project that share every fact of RE-483's probe (an unset form that is shown and unbound, one void, one nested instance) are two families:

| Family | Windows | Symbols | Geometry tags in the map | Written | Not written |
| --- | ---: | ---: | --- | ---: | ---: |
| 1734290 | 8 | 8 | 33 | 8 | 0 |
| 1759705 | 7 | 4 | 13 | 0 | 7 |

So the 8 and 7 of RE-483 are not mixed within any family: the instance does not decide it. The 49 windows with a bound unset form of ambiguous caption are four families: 1704283 (36 windows, 30 symbols, 21 tags) and 1749842 (8, 5 symbols, 21 tags) write it on every window; 1792456 (4, 13 tags) and 1665347 (1, 21 tags) on none. The 4 whose bound unset form resolves are one family, 1672073 (13 tags), with none written. Of the 4 fact groups the project has, the 2 with both outcomes are constant within a symbol and within a family, and not within an unset tag count.

**Row 4: the nested instances.** Windows with every nested instance shown and none bound: written 52, not written 12 (spent project); not written 54 (Warmond). A nested instance bound to a parameter: not written 4 (spent project), 263 (BIM Whale windows), 14 (Warmond doors), 10 (BIM Whale doors). No nested instance: not written 22 (Frigento doors). No nested instance is hidden by its stored `Visible` on any pre-2024 project. So the nested instances' visibility does not separate: the same state, all shown and none bound, is written on 52 of 64 windows of one project and on none of 54 of another.

**Row 5: the unset tags.** The elements by the number of unset tags of the symbol's map, written / not written:

| Project | 1 unset | 2 unset | 3 or more unset |
| --- | --- | --- | --- |
| Spent project, windows | 0 / 0 | 41 / 1 | 11 / 15 |
| Frigento, doors | 0 / 1 | 0 / 21 | 0 / 0 |
| Warmond, windows | 0 / 0 | 0 / 24 | 0 / 30 |
| Warmond, doors | 0 / 0 | 0 / 0 | 0 / 14 |
| BIM Whale, windows | 0 / 0 | 0 / 0 | 0 / 263 |
| BIM Whale, doors | 0 / 0 | 0 / 0 | 0 / 10 |

No count separates: two unset tags is written on 41 of 42 spent windows and on none of 45 elsewhere; three or more is written on 11 of 26 and on none of 317. Inside one family the count varies and the result does not: family 1704283 writes it on windows with 2, 9 and 13 unset tags of 21, and family 1792456 writes it on none with 7 and 11 unset tags of 13.

**Total tags, read from the spent project's lines only.** The map's total number of geometry tags (not counted by the scorer) falls with the families of the spent project: 33 tags, 8 of 8 written; 21 tags, 44 of 45 written; 13 tags, 0 of 15. One window breaks it (family 1665347, 21 tags, 2 unset, not written), and the total is not counted on the other projects, so it is a lead and not a rule.

**Row 6: nothing moves.** No `src/` change.

## 4. The registered rows

Registered on #1603 before the run (2026-11-15, comment 6894163538), read from Measure 43714791546.

1. **Type.** Reported (table above): no drawn symbol is mixed on any project, with 43 symbols of the spent project written on all of their elements and 13 on none. Every symbol is all-or-none; most hold one window, so this alone does not rule out an instance-level fact, and the family row does.
2. **Family.** Reported: no family is mixed, 3 of the spent project's 7 write it on all of their windows and 4 on none. The families hold several windows each (36 in the largest) and none is mixed, so within a family the instance does not decide it; a family mixed where its symbols are not would have said the type is the unit, and none is.
3. **The 15.** Reported: two families, 8 written of one and 7 not written of the other.
4. **Nested instances.** Reported: no separation (above).
5. **Unset tags.** Reported: no count is right on every element of every pre-2024 project, so no rule is proposed.
6. **Nothing moves.** Met: no `src/` change.

## 5. What this decides and does not

- **Decided.** `<Unnamed>` before 2024 is a property of the family of the door or window, not of the instance and not of the number of unset tags: every pre-2024 family is all-or-none, and the 15 windows with identical host-form facts differ by family.
- **Decided.** No fact read so far says which families: not the host family's forms (RE-482, RE-483), not the nested instances' stored or bound `Visible`, not the unset tag count. The next read is of what a family holds that these do not: the nested families' own forms and materials (every window of the spent project and of BIM Whale nests an instance, and three for 4 of the spent project's), and the family's category and subcategory settings.
- **Not decided.** Whether the exporter's release or the IFC schema decides it: the reference of each project is from the save's own release, and the spent project is the only one with a family that writes it (RE-483, Row 4).
- **A lead, not a rule.** The map's total geometry tags (13, 21, 33) follow the written families except one window. The scorer does not count it on the other projects, so the next run does (it can be refuted by any other project's families with 21 or more tags and none written).

## 6. Reproduce

```bash
# the probe at fa38cbf51a8c, the samples and every held-out project (the held-out counts need the scrub of this change, which --ref main holds only once it lands)
gh workflow run measure.yml --ref main -f ref=fa38cbf51a8cf51f0c3f1d5785a609159f305ac4 -f base=none -f samples=true -f holdout=true \
  -f scorers=none -f probe=probe_1603_unnamed_by_type_before_2024 -f probe_scorer=window_door_unnamed_by_type_vs_ifc
# each model's probe.txt holds the `wtype` lines; the spent project's probe_score.txt lists every counted element (python3 tools/re/window_door_unnamed_by_type_vs_ifc.py --list); a held-out model's counts.txt holds the `unnamed by type`, `facts`, `unset tags` and `nested before 2024:` lines
```

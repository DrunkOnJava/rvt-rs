# RE-487: before Revit 2024 neither the type map's total geometry tags nor the maps of the types a family's nested instances draw say which families write `<Unnamed>`: two of the spent Revit 2018 project's three 21-tag window families write it and one does not, and 317 doors and windows of 21 to 81 tags on two other projects write it on none

**Date:** 2026-11-15
**Issues:** #1603 (Revit 2015 to 2023: the 70 windows of the spent Revit 2018 project lack Revit's materials); follows RE-485 (`<Unnamed>` is constant within each of the spent project's 7 window families, 3 writing it on all of their windows and 4 on none), RE-483, RE-482, RE-481.
**Artefacts:** the spent Revit 2018 project (the held-out save of it carries the same lines and is counted once), Frigento (Revit 2020), Warmond (Revit 2021) and BIM Whale (Revit 2021), 431 doors and windows whose type map holds geometry mapped to no material and to which Revit gives materials. The spent project's and BIM Whale's references are IFC2X3 exports and Warmond's is an IFC4 export passed through BIMServer.center's preprocessor, all counted apart and never removing a value; Frigento's 22 doors are the only ones measured against an unmodified Revit IFC4 export.
**Probe and scorer:** `probe_1603_unnamed_nested_types_before_2024` and `tools/re/window_door_unnamed_nested_types_vs_ifc.py` at `7f2a241293c8` (Measure [43716973206](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43716973206)), archived under `refs/archive/2026-11-15/cited/`. No `src/` change.
**Status:** negative for a rule. The map's total geometry tags follow the spent project's 7 window families except one (13 tags: 0 of 15 windows written; 21: 44 of 45; 33: 8 of 8), but the one window of a third 21-tag family writes none beside the 44 windows of two 21-tag families that write it on all of theirs, and the 317 doors and windows of Warmond and BIM Whale (both counted apart) whose maps hold 21 to 81 tags write it on none; Frigento's two families hold 3 and 7 tags, so it neither supports nor refutes the total. Every window of the spent project nests an instance whose type's map holds an unset tag, as do all 341 doors and windows of Warmond and BIM Whale: the state is the same on the 3 written families and on 3 of the 4 not-written ones (the fourth nests three instances), so it does not separate either. With RE-481 to RE-485 and this report, none of the facts read of a pre-2024 door or window's family says which families write `<Unnamed>`; nothing is written.

## 1. The question

RE-485 found `<Unnamed>` constant within each of the spent project's 7 window families before 2024 and the type map's total geometry tags (13, 21, 33) following those families except one window, and left two cheap reads: the total on the other pre-2024 projects, where any family with 21 or more tags and none written drops the lead; and the maps of the types a family's nested instances draw, which every window of the spent project and of BIM Whale nests. Does either say which families write it?

## 2. What the probe and the scorer count

For each door and window the export writes, the probe prints the drawn symbol's and the family's ElementIds, the number of geometry tags the symbol's `m_geomTag2MaterialId` holds and how many map to no material, and of the family's nested family instances how many it has, how many draw a type of the family's document whose map was read and how many of those maps hold a geometry tag mapped to no material. The scorer counts, per project and IFC class, for the elements whose type map holds geometry mapped to no material and to which Revit gives materials, `<Unnamed>` written or not by the symbol's total tags, and by whether the family has no nested instance, nested types read with none holding an unset tag, a nested type holding one, or a nested type not read. Each record of a family's document is read once, from its last copy. Counts only for held-out projects. The lines are printed for every release; this report reads those of Revit 2015 to 2023 (a Revit 2026 held-out house in the run, 13 doors and 12 windows with `<Unnamed>` written on all 25, is RE-481's).

## 3. Measured

Measure 43716973206, 84 models, the spent project listed per element.

**Row 1: total tags.** Per project and IFC class, the elements by the total geometry tags of the drawn symbol's map, `<Unnamed>` written / not written:

| Project | Class | Elements | Total tags: written / not written |
| --- | --- | ---: | --- |
| Spent Revit 2018 (IFC2X3) | windows | 68 | 13: 0 / 15; 21: 44 / 1; 33: 8 / 0 |
| Frigento (Revit 2020) | doors | 22 | 3: 0 / 1; 7: 0 / 21 |
| Warmond (Revit 2021, preprocessed, counted apart) | windows | 54 | 19: 0 / 24; 31: 0 / 6; 38: 0 / 24 |
| Warmond (counted apart) | doors | 14 | 21: 0 / 12; 42: 0 / 2 |
| BIM Whale (Revit 2021, IFC2X3, counted apart) | windows | 263 | 81: 0 / 263 |
| BIM Whale (counted apart) | doors | 10 | 48: 0 / 10 |

On the spent project the total is 13 on 3 families (7, 4 and 4 windows, none written), 33 on one (8 windows, all written) and 21 on three: 1704283 (36 windows) and 1749842 (8) write it on all of theirs, and 1665347, which holds one window, writes it on none. So the total fails on that project too, by one window of one family. Outside it, 317 doors and windows hold 21 or more tags, in 6 different totals, and have it written on none: Warmond's 44 (doors of 21 and 42 tags, windows of 31 and 38) and BIM Whale's 273 (doors of 48, windows of 81). Both projects are counted apart, so they never remove a value; a rule that wrote `<Unnamed>` from the total would write it on all 317, where their references have none. Frigento, the one unmodified Revit IFC4 sample, holds 3 and 7 tags only, so it has nothing at 21 or more to test the lead with. The lead of RE-485 is dropped.

**Row 2: the nested instances' own types.** Per project and IFC class, the elements by whether any nested instance of the family draws a type whose map holds a geometry tag mapped to no material, `<Unnamed>` written / not written (- is no element):

| Project | Class | Elements | No nested instance | Nested types read, none with an unset tag | A nested type with an unset tag | A nested type not read |
| --- | --- | ---: | --- | --- | --- | --- |
| Spent Revit 2018 (IFC2X3) | windows | 68 | - | - | 52 / 16 | - |
| Frigento (Revit 2020) | doors | 22 | 0 / 22 | - | - | - |
| Warmond (counted apart) | windows | 54 | - | - | 0 / 54 | - |
| Warmond (counted apart) | doors | 14 | - | - | 0 / 14 | - |
| BIM Whale (counted apart) | windows | 263 | - | - | 0 / 263 | - |
| BIM Whale (counted apart) | doors | 10 | - | - | 0 / 10 | - |

All 68 windows of the spent project are in one bucket, with 52 written and 16 not, so the bucket does not separate. By family:

| Family | Windows | Symbols | Total tags | Nested instances | Their types with an unset tag | Written | Not written |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1704283 | 36 | 30 | 21 | 1 | 1 | 36 | 0 |
| 1734290 | 8 | 8 | 33 | 1 | 1 | 8 | 0 |
| 1749842 | 8 | 5 | 21 | 1 | 1 | 8 | 0 |
| 1759705 | 7 | 4 | 13 | 1 | 1 | 0 | 7 |
| 1672073 | 4 | 4 | 13 | 3 | 2 | 0 | 4 |
| 1792456 | 4 | 4 | 13 | 1 | 1 | 0 | 4 |
| 1665347 | 1 | 1 | 21 | 1 | 1 | 0 | 1 |

The same state, one nested instance whose type's map holds an unset tag, is on the 3 written families (52 windows) and on 3 not-written ones (12 windows); the one family in another state (three nested instances, two of whose types' maps hold one) is not written. Outside the spent project the bucket holds Warmond's 68 and BIM Whale's 273 with none written, and Frigento's 22 doors nest nothing. So the nested types' maps do not say which families write it.

**Nested types read.** Every nested instance's type map was read on the four projects: 9 of 9 in the spent project's families (8 holding an unset tag), 5 of 5 in Frigento's (none), 27 of 27 in Warmond's (12) and 20 of 20 in BIM Whale's (7), the nested instances of all the families of each project's document and not only its doors and windows; none is unread on any of the 44 models of Revit 2015 to 2023 (325 of 325).

**Records held in several partitions.** As in RE-483, each record of a family's document is read from its last copy and the records held in more than one partition are counted: of the 3,253 `FamilyInstance` and `FamilySymbol` records read over the 84 models none is held in more than one.

**Row 4: nothing moves.** No `src/` change.

## 4. The registered rows

Registered on #1603 before the run (2026-11-15, comment 6894610678), read from Measure 43716973206.

1. **Total tags, every project.** Reported (above): the lead drops. The other projects hold 6 totals of 21 to 81 tags, 317 elements, with none written, and one window of a 21-tag family of the spent project writes none beside 44 that do; Frigento's totals (3, 7) say nothing of it.
2. **The nested instances' own types.** Reported: no separation; the spent project's 68 windows and 341 doors and windows elsewhere hold a nested type with an unset tag, written on 52 of the 68 and on none of the 341.
3. **A rule.** Not met: no rule is right on every element of every pre-2024 project, so nothing is written. No fact read separates the 16 families (7 on the spent project, 9 elsewhere), so the question stays closed for Revit 2015 to 2023 unless a new read or a new project does.
4. **Nothing moves.** Met: no `src/` change.

## 5. What this decides and does not

- **Decided.** The type map's total geometry tags do not decide `<Unnamed>` before 2024. They fit 6 of the spent project's 7 window families and fail on the seventh (one window), and 317 doors and windows of two other projects, both counted apart, hold 21 to 81 tags and write none.
- **Decided.** The maps of the types a family's nested instances draw do not decide it: the written and the not-written families of the spent project hold the same state, and so do the 341 doors and windows of the two other projects that nest an instance.
- **Decided.** With RE-481 to RE-485 and this report, none of the facts read of a pre-2024 door or window's family (its forms, their stored and bound `Visible`, voids, nested instances and the maps of their types, the symbol's unset and total tags) says which families write `<Unnamed>`. No rule is written, and `<Unnamed>` before 2024 stays as the scan gives it.
- **Not decided: what does.** Every family that writes it is on one project, the Revit 2018 IFC2X3 export of the spent project, so whether the exporter's release or schema is the cause cannot be told from these files (RE-483, Row 4). Not read: the nested families' own forms and materials, and the family's category and subcategory settings (the object-style material RE-481 names). A second pre-2024 project whose families write it, above all an unmodified Revit IFC4 export, would reopen the question.

## 6. Reproduce

```bash
# the probe at 7f2a241293c8, the samples and every held-out project (the held-out counts need the scrub of this change, which --ref main holds only once it lands)
gh workflow run measure.yml --ref main -f ref=7f2a241293c8ed83f619ab294c95aa552cb27e06 -f base=none -f samples=true -f holdout=true \
  -f scorers=none -f probe=probe_1603_unnamed_nested_types_before_2024 -f probe_scorer=window_door_unnamed_nested_types_vs_ifc
# each model's probe.txt holds the `wnest` lines and the `nested types read:` and `family record copies:` lines; the spent project's probe_score.txt lists every counted element (python3 tools/re/window_door_unnamed_nested_types_vs_ifc.py --list); a held-out model's counts.txt holds the `unnamed by total tags before 2024:`, `unnamed by nested types before 2024:`, `nested types read:` and `family record copies:` lines
```

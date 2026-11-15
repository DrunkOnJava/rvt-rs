# RE-482: before Revit 2024 none of the facts read here of a door or window's family forms (shown, hidden, void and nested) separates the doors and windows whose set holds `<Unnamed>` from those whose set does not: it is written on none of 363 outside the spent Revit 2018 project, and on 52 of its 68 windows by no category

**Date:** 2026-11-15
**Issues:** #1603 (Revit 2015 to 2023: the 70 windows of the spent Revit 2018 project lack Revit's materials); follows RE-481 (the Revit 2024 rule for `<Unnamed>` is wrong before 2024), RE-479 (the family forms decode before 2024), RE-204 to RE-206 (when Revit writes `<Unnamed>`), RE-149 (geometry mapped to no material is `<Unnamed>`).
**Artefacts:** the spent Revit 2018 project, the 18 held-out projects of Measure (counts only) and the 65 reference models and Autodesk samples, 84 models. The references of the spent project and of BIM Whale's `AdvancedProject` are IFC2X3 exports, and Warmond's is an IFC4 export passed through BIMServer.center's preprocessor; all three are counted apart and never remove a value, so only Frigento's 22 doors are measured against an unmodified Revit IFC4 export (the schema and preprocessing of each are RE-483's).
**Probe and scorer:** `probe_1603_unnamed_before_2024` and `tools/re/window_door_unnamed_vs_ifc.py` at `cf5a70284a41` (Measure 43705534691), archived under `refs/archive/2026-11-15/cited/`. No `src/` change.
**Status:** negative for a rule, positive for the reader of `Visible`. Of the 431 doors and windows of Revit 2015 to 2023 whose drawn symbol's map holds geometry mapped to no material and to which Revit gives materials, Revit's set holds `<Unnamed>` on 52, all windows of the spent project. On the other 363 (Frigento's 22 doors, Warmond's 68 and BIM Whale's 273, the last two counted apart) it is written on none, whether the family shows a form with no material (100 of them) or nests an instance (263). On the spent project it is written on 44 of the 45 windows whose family hides every form with no material (stored `Visible` 0), on 8 of the 19 that show one, and on none of the 4 that show one and hide another, so no category separates, and every window of the project nests an instance whose own forms the facts do not cover. Forms with a stored `Visible` of 0 are found on saves before 2024 exactly as on their 2024 saves.

## 1. The question

RE-481 found that the Revit 2024 rule (`<Unnamed>` where the family shows a form with no material) is wrong before 2024: on the spent project Revit 2018 writes ` <Unnamed>` on 37 of 38 windows whose family shows none and on 8 of 21 whose family shows one. Which facts of a family's forms, if any, separate the doors and windows Revit gives `<Unnamed>` from those it does not, on each project, and is a form hidden by a stored `Visible` of 0 found before 2024 as it is from it (the gap of RE-481's control)?

## 2. What the probe and the scorer count

For each door and window the export writes, the probe prints whether the drawn symbol's map holds geometry mapped to no material (-1) and the facts of its family's forms: the forms with no material that are not voids and whose stored `Visible` is not 0 (shown), those whose is 0 (hidden), the voids with no material, the nested instances, the forms of any material with a stored `Visible` of 0, and the forms with no material whose `Visible` is bound to a family parameter. Per model it prints the family forms by stored `Visible` value. The scorer takes the elements whose map holds a -1 and to which Revit gives materials, and counts whether Revit's set holds `<Unnamed>` (a leading space dropped) by the family's category: an unset form shown; none shown, one hidden; only voids with none; no form with none, and a nested instance; no form with none; the family's document not read. Counts only for held-out projects.

## 3. Measured

**Row 1: reach.** Every one of the 431 doors and windows of Revit 2015 to 2023 has its family read (`family not read` is 0 in every project).

**Row 2: `Visible`.** Forms with a stored `Visible` of 0, over the models of each release: 2015 2, 2016 2, 2017 2, 2018 6, 2019 2, 2020 2, 2021 2, 2022 2, 2023 2; 2024 20, 2025 17, 2026 4, 2027 4 (the stored value is 0 or 1 on every form: no other value, none unread). The architectural basic sample holds 2 on each of its 13 saves, 2015 to 2027; every other sample project holds none on any of its saves, and the counts of every sample project are equal across its saves (the structural basic project's 2015 save, which lists fewer families, apart). The spent project's families hold 4.

**Rows 3 and 4: the cross-tab.** Doors and windows whose map holds geometry mapped to no material, by the family's category, Revit's `<Unnamed>` written / not written:

| Project | Category | Written | Not written |
| --- | --- | ---: | ---: |
| Spent Revit 2018 (IFC2X3), windows | an unset form shown | 8 | 15 |
| | none shown, one hidden | 44 | 1 |
| Frigento (Revit 2020), doors | an unset form shown | 0 | 22 |
| Warmond (Revit 2021, preprocessed, counted apart), windows | an unset form shown | 0 | 54 |
| Warmond (counted apart), doors | an unset form shown | 0 | 14 |
| BIM Whale Revit 2021 (IFC2X3, counted apart), doors | an unset form shown | 0 | 10 |
| BIM Whale Revit 2021 (IFC2X3, counted apart), windows | no form with none, a nested instance | 0 | 263 |

The spent project's 68 windows split further: 19 with an unset form shown and none hidden (written 8, not 11), 4 with one shown and one hidden (written 0, not 4), 45 with none shown and one hidden (written 44, not 1). All 68 families nest an instance; 64 of the 68 hold a void. The spent project's other 64 doors map nothing to a material and have no material in Revit's export.

**From Revit 2024.** The 25 doors and windows of Revit 2024 to 2027 projects (with Core Interior and RE1 Architecture) whose map holds a -1 and that Revit gives materials: an unset form shown 13, only voids with none 12; Revit writes `<Unnamed>` on all 25.

## 4. The registered rows

Registered on #1603 before the run (2026-11-15, comment 6893137303), read from Measure 43705534691.

1. **Reach.** Met: 431 of 431, no family unread.
2. **Visible.** Met: forms with a stored `Visible` of 0 are found on saves before 2024, 2 to 6 per release, and equal on every save of a sample project that has one on both sides of 2024 (the architectural basic sample, 2 on 13 saves).
3. **Cross-tab.** Not met as a rule: no category has `<Unnamed>` written on every element of it on every project, or on none of them on every project. "An unset form shown" has it written on 8 of 23 spent windows and on none of 100 elsewhere; "none shown, one hidden" exists on one project.
4. **The spent project.** Reported above: no category separates; 44 of the 45 with a hidden unset form and none shown have it written, 8 of the 19 with one shown and none hidden, none of the 4 with both.
5. **Nothing moves.** Met: no `src/` change.

## 5. What this decides and does not

- **Decided.** None of the facts read here of the host family's own forms (each form's material, whether it is a void, its stored and bound `Visible`, and the nested instances) decides `<Unnamed>` before 2024; the forms' subcategory or object-style materials were not read, and RE-481 names the object-style material as a candidate. `<Unnamed>` before 2024 stays as the scan gives it; no reader writes a rule for it.
- **Decided.** A form hidden by a stored `Visible` of 0 is found on saves before 2024 as on the 2024 saves of the same projects, so RE-481's control now exercises it (the architectural basic sample's two forms). RE-481's hidden-form result stands on that: the 69 "drawn more" are not materials only hidden forms hold, as read.
- **Not decided: what writes `<Unnamed>` before 2024.** Revit 2020 and 2021 exports of three projects (Frigento's IFC4, Warmond's IFC4 through a preprocessor and BIM Whale's IFC2X3, the last two counted apart; RE-483 read each export's schema and release) write it on none of 363 elements whose families show a form with no material or nest an instance; the Revit 2018 IFC2X3 export of the spent project writes it on 52 of 68. RE-483 found each reference's release the same as its save's, so the exporter's release cannot be told from the save's with these files; and every window of the spent project and of BIM Whale nests an instance whose own forms are not in these facts.
- **Not decided: the 2024 rule for a void.** On Revit 2024 to 2027, 12 of 12 doors and windows whose only form with no material is a void have `<Unnamed>` written; `native_family_forms` does not count a void. The export keeps the raw map there (RE-481: 287 of 289 equal before and after the rules), so nothing is wrong today; it is not measured whether a rule that counted voids would be better.
- **Not measured: Revit 2015 to 2018 exports other than the spent project's.**

## 6. Reproduce

```bash
# the probe at cf5a70284a41, the samples and every held-out project (the held-out counts need the scrub of this change, which --ref main holds only once it lands)
gh workflow run measure.yml --ref main -f ref=cf5a70284a418d195a09b25cceae059cc295b54a -f base=none -f samples=true -f holdout=true \
  -f scorers=none -f probe=probe_1603_unnamed_before_2024 -f probe_scorer=window_door_unnamed_vs_ifc
# each model's probe.txt holds the `wfam` lines and the `family visible values:` line; the spent project's probe_score.txt lists every counted element; a held-out model's counts.txt holds the `unnamed before 2024:` lines
```

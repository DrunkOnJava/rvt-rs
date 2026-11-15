# RE-481: the rules RE-204 to RE-206, read from the family forms, bring the drawn symbol's map to Revit's set on 1 of the 387 doors and windows of Revit 2015 to 2023 that differ, and take 37 of the 459 that agree away from it

**Date:** 2026-11-15
**Issues:** #1603 (Revit 2015 to 2023: the 70 windows of the spent Revit 2018 project lack Revit's materials); follows RE-478 (the drawn symbol's map, and the rules it lacks), RE-479 (the family forms decode on every release, the material in the older id shape before 2024), RE-204 to RE-206 (when Revit writes `<Unnamed>`, and the materials only hidden forms hold), RE-149 (geometry mapped to no material is `<Unnamed>`).
**Artefacts:** the spent Revit 2018 project, the 18 held-out projects of Measure (counts only; the held-out save of the spent project is counted once) and the 65 reference models and Autodesk samples, 84 models, each exported from main and from the head with no export changed (`ifc.sha256` 84 of 84 equal, `diagnostics.json` 66 of 66).
**Probe and scorer:** `probe_1603_family_rules_pre2024` and `tools/re/window_door_family_rules_vs_ifc.py`, with `native_family_forms::family_forms_in`, at `bf0967e8d33e` (Measure 43703452041), archived under `refs/archive/2026-11-15/cited/`.
**Status:** negative, with one limit on the control (section 3). Applied as the Revit 2024 and later reader applies them (the map's materials minus the family's hidden-only materials, `<Unnamed>` only where the family shows geometry with no material), the rules read from the forms of every family of the 846 doors and windows of Revit 2015 to 2023 (846 of 846 give a decode) leave the 299 "unset material only" doors and windows as they are but one, take none of the 79 "drawn more" to equal (10 lose their extra names and gain a `<Unnamed>` Revit does not write), and take 37 of the 45 spent-project windows whose map agrees with Revit's once its spelling of the unset material is dropped to "unset material only": Revit 2018 writes ` <Unnamed>` on windows whose family shows no unset form (the geometry mapped to no material is a void's, or a hidden form's, or no form's). The rules are the Revit 2024 exporter's. Before 2024 `<Unnamed>` and the hidden-only materials follow something else, which the family forms as read here do not give; whether a form hidden by a stored `Visible` of 0 is read before 2024 is the one thing the control does not test.

## 1. The question

RE-478 found that the drawn symbol's map is Revit's material set on 414 of the 846 doors and windows of the 8 projects of Revit 2015 to 2023 (459 with Revit 2018's spelling of the unset material dropped), and that the 387 others differ in whether `<Unnamed>` is written (299), by names Revit's set does not hold (79) and by one material (9). RE-479 found the families' forms readable in the older id shape. Do the rules that decide those two things on Revit 2024 and later (RE-204 to RE-206) decide them before it?

## 2. What the probe and the scorer count

For each door and window the export writes, the probe prints the drawn symbol's map (as RE-478) and the set after the rules, as `refine_type_materials` applies them: the map's materials minus the materials only hidden forms of the family hold (`FamilyForms::hidden_only`), and `<Unnamed>` where the map has geometry mapped to no material and the family shows it (`shows_unset` not false; true where a form that is neither a void nor hidden has no material; false where no such form is shown (the family has no form with no material, or each has a stored `Visible` of 0; a void's is not counted); unknown where the family nests an instance or a form did not decode). The scorer classes both against Revit's set in RE-478's kinds and counts the moves between them. Counts only for held-out projects (`tools/ci/scrub-holdout-results.sh` keeps the scorer's lines).

## 3. Measured

**Reach (row 1).** Every door and window Revit gives materials has a map and a family whose forms decode: 846 of 846 before 2024 (525 windows, 321 doors), 289 of 289 from it (it includes Core Interior and RE1 Architecture). `shows_unset` before 2024: true 123, false 437, unknown 286.

**Before 2024, before and after the rules.** Doors and windows of the 8 projects of Revit 2015 to 2023 (equal, spelled apart, unset material only, Revit more, drawn more):

| Project | Elements | Before the rules | After the rules |
| --- | ---: | --- | --- |
| Spent Revit 2018, windows | 70 | 2, 45, 14, 9, 0 | 3, 8, 50, 9, 0 |
| Spent Revit 2018, doors | 4 | 4, 0, 0, 0, 0 | 4, 0, 0, 0, 0 |
| Held-out Revit 2020, windows | 29 | 29, 0, 0, 0, 0 | same |
| Held-out Revit 2020, doors | 27 | 4, 0, 22, 0, 1 | same |
| Held-out C (Revit 2020), windows and doors | 21 and 16 | all equal | same |
| Held-out A (Revit 2021), windows | 110 | 56, 0, 0, 0, 54 | same |
| Held-out A (Revit 2021), doors | 140 | 126, 0, 0, 0, 14 | same |
| BIM Whale Revit 2021, windows | 263 | 0, 0, 263, 0, 0 | same |
| BIM Whale Revit 2021, doors | 126 | 116, 0, 0, 0, 10 | 116, 0, 10, 0, 0 |
| Revit 2022 and 2023, three projects | 1 to 12 and 1 to 4 each | all equal | same |

In all: equal 414 to 415, spelled apart 45 to 8, unset material only 299 to 345, drawn more 79 to 69, Revit more 9 to 9, apart 0. The moves: unset material only to equal 1 (a spent window); spelled apart to unset material only 37 (spent windows); drawn more to unset material only 10 (BIM Whale's doors). Nothing that was equal as written (414) is not equal after.

**From 2024.** The 289 doors and windows (178 doors, 111 windows) are 287 equal before and after the rules, and 2 drawn more (a Revit 2026 project's doors) before and after: the rules are the ones that reader applies, and the raw map already agrees.

**The control.** The sample projects have saves on both sides of 2024, with the same ElementIds. For each of the five that hold doors or windows (the architectural basic and advanced samples, the MEP basic and advanced samples) the `wrule` lines of every save before 2024 (2015 to 2023, 2015 and 2019 for the MEP advanced) equal the 2024 save's on every Tag: the set after the rules, `forms`, `shows_unset` and `hidden` (124, 33, 124 and 124 Tags). `shows_unset` is true on 107 of the 124 Tags of each MEP project and false on the others and on every architectural Tag, so the unset-form read is exercised and agrees; `hidden` is 0 on every Tag, so the read of a form hidden by a stored `Visible` of 0 is not exercised, on either side.

**Why the 37.** The spent project's families: 40 forms, 22 with no material (16 extrusions, 6 sweeps), 6 voids. For 47 of its 70 windows `shows_unset` is false (the family has no unset form that is neither a void nor hidden: its geometry mapped to no material is a void's, a hidden form's, or none is in its forms) and the rule writes no `<Unnamed>`; on 38 of them the map has geometry mapped to no material and the names are otherwise Revit's, and Revit 2018 writes ` <Unnamed>` on 37 of the 38. For the 23 where it is true the rule writes it; on 21 of them the names are otherwise Revit's, and Revit writes ` <Unnamed>` on 8 and none on 13. So on this project the visibility the 2024 rule reads decides nothing and points the wrong way: where the family shows a visible form with no material Revit 2018 mostly writes no `<Unnamed>` (13 of 21), and where it shows none it nearly always does (37 of 38). Why is not measured, and the probe does not say which of the three `false` is.

**BIM Whale and the Revit 2020 doors.** All 263 of BIM Whale's windows have `shows_unset` unknown (the family nests an instance), the rule keeps `<Unnamed>` as the map has it, and Revit writes none: 263 "unset material only" before and after. 22 of the held-out Revit 2020 project's 27 doors have `shows_unset` true and 22 are "unset material only" before and after the rules.

**The "drawn more" doors and windows.** 69 are unchanged: 54 windows and 14 doors of Held-out A, 1 door of the Held-out Revit 2020 project. The names the map holds and Revit's set does not are not materials only the hidden forms the reader finds hold.

## 4. The registered rows

Registered on #1603 before the run (2026-11-15, comment 6892852422), read from Measure 43703452041.

1. **Reach.** Met: 846 of 846 before 2024; the spent project's 11 of 11 families.
2. **Unset material.** Not met. Of the 299, 1 is equal after the rules (a spent window); 298 are unchanged.
3. **Hidden forms.** Not met, with the control's limit: of the 79, none is equal after; 10 are unset material only (the extra names gone, a `<Unnamed>` written that Revit does not write), 69 unchanged. The 69 keep their extra names, so they are not materials only forms the reader finds hidden hold; whether it finds a hidden form before 2024 is not tested (section 3).
4. **No regression.** Not met. Of the 459 equal as written or with the spelling apart, 414 stay equal and 37 (spent windows, spelled apart) become unset material only.
5. **The nine windows.** Met: unchanged.
6. **Nothing moves.** Met: `ifc.sha256` 84 of 84 and `diagnostics.json` 66 of 66 equal to main.

## 5. What this decides and does not

- **Decided.** The `<Unnamed>` rule (RE-204) as the Revit 2024 and later reader applies it does not carry to Revit 2015 to 2023: it would be wrong on 37 of the 459 elements that agree today and right on 1 of the 387 that do not, on a `shows_unset` the control reads identically on every pre-2024 save of five projects. `native_family_forms` is not widened to read it in an export before 2024; `family_forms_in` is a reader of the research probes.
- **Not decided: the hidden-form rule (RE-205, RE-206) before 2024.** The 69 "drawn more" are unchanged by it, and it moves 10 to a `<Unnamed>` Revit does not write; but the control has no form hidden by a `Visible` of 0 on the 2024 side (`hidden` is 0 on every Tag of the five projects), so that the reader finds such a form on a save before 2024 is not tested. The next probe counts forms by stored `Visible` and says why `shows_unset` is false.
- **Not decided: what writes `<Unnamed>` before 2024.** On the spent project (IFC2X3, counted apart) it is written on 52 of the 70 windows, 37 of 38 where the family shows no unset form and 8 of 21 where it shows one. On BIM Whale (IFC2X3, counted apart) and on the held-out Revit 2020 doors it is not written on 263 windows and 22 doors, though the family shows a visible form with no material or nests an instance. Whether the export's IFC version, the exporter's release, the object-style material a form with no material takes, or the project decides it is not measured. The next measure is the cross-tab of the map's geometry mapped to no material, whether Revit's set holds `<Unnamed>`, and what the family shows, by project.
- **Not decided: the 69 "drawn more".** The extra names are not materials the reader finds only hidden forms hold; what they are is not measured.
- **Not decided: the nine windows' `LKS_VEN_MADERA`** (RE-478).
- **Not measured: Revit 2015 to 2018 exports other than the spent project's.**

## 6. Reproduce

```bash
# the probe at bf0967e8d33e against main, the samples and every held-out project (the held-out counts need the scrub of this change, which --ref main holds only once it lands)
gh workflow run measure.yml --ref main -f ref=bf0967e8d33e5d10e90f2ed18a4081424e10e591 -f base=main -f samples=true -f holdout=true \
  -f scorers=none -f probe=probe_1603_family_rules_pre2024 -f probe_scorer=window_door_family_rules_vs_ifc
# each model's probe.txt holds the `wrule` lines; the spent project's probe_score.txt lists the elements that differ; a held-out model's counts.txt holds the `family rules:` lines
```

# RE-483: before Revit 2024 a form's `Visible` bound to a family parameter does not decide `<Unnamed>` either: 15 of the spent Revit 2018 project's 68 windows share every fact read, effective visibility included, and 8 have `<Unnamed>` written

**Date:** 2026-11-15
**Issues:** #1603 (Revit 2015 to 2023: the 70 windows of the spent Revit 2018 project lack Revit's materials); follows RE-482 (none of the facts read of a family's forms by their stored `Visible` separates `<Unnamed>` written from not), RE-481, RE-479, RE-204 to RE-206, RE-149.
**Artefacts:** the spent Revit 2018 project, the 18 held-out projects of Measure (counts only; the held-out save of the spent project is counted once) and the 65 reference models and Autodesk samples, 84 models. The references of the spent project and of BIM Whale's `AdvancedProject` are IFC2X3 exports, and Warmond's is an IFC4 export passed through BIMServer.center's preprocessor; all three are counted apart and never remove a value. Frigento's 22 doors are the only elements here measured against an unmodified Revit IFC4 export.
**Probe and scorer:** `probe_1603_unnamed_bound_before_2024` and `tools/re/window_door_unnamed_bound_vs_ifc.py` at `04c2dfd9d977` (Measure [43714011584](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43714011584)), which reads each record of a family's document once, from its last copy, and counts the records held in several partitions; the first run, at `26ae3aeeb5a6` (Measure 43711618548), read every copy, and its per-element lines and cross-tab are the same as the second's. Both archived under `refs/archive/2026-11-15/cited/`. No `src/` change.
**Status:** negative for a rule. Reading a `Visible` bound to a family parameter from that parameter's value does not make `<Unnamed>` follow what the family's forms show. On the spent project's 68 windows: 15 share every fact read (an unbound unset form that is shown, one void, one nested instance) and 8 have `<Unnamed>` written and 7 not; the 4 whose bound unset form resolves have none written; the other 49 have a bound unset form whose caption names two parameters (49 of the project's 53 bound unset forms), so its value is not taken, and 44 of them have `<Unnamed>` written and 5 not. Elsewhere in Revit 2015 to 2023 (Frigento's 22 doors, Warmond's 68 doors and windows and BIM Whale's 10 doors and 263 windows, the last two counted apart) it is written on none of 363, and all 50 pairs of a window or door and a bound unset form of its family in Warmond and BIM Whale resolve.

## 1. The question

RE-482 found that no category of the host family's forms by their stored `Visible` separates the doors and windows whose set holds `<Unnamed>` from those whose set does not. The spent project's windows split by a fact it did not use: 44 of the 45 whose only unset form is stored hidden have that form bound to a family parameter and `<Unnamed>` written, and the 8 whose bound unset form is stored shown have none. A stored `Visible` says nothing of what a bound form shows. Does `<Unnamed>` follow the form's effective visibility, the bound parameter's value on the instance, else on its type?

## 2. What the probe and the scorer count

For each door and window the export writes, the probe prints the family's forms with no material that are not voids: how many are effectively shown, hidden or unresolved, how many are bound to a family parameter and how many of those resolve, and how many it left unresolved because the parameter's caption names two parameters (two family parameters of the family with one caption, or two project parameters the instance and its type set with one caption and different ids). A form's effective visibility is its stored `Visible` where it is not bound, else the value of the bound parameter on the instance, else on its type, matched by caption as `instances_without_body` matches it. The scorer counts, for the elements whose type map holds geometry mapped to no material and to which Revit gives materials, `<Unnamed>` written or not by the family's category, and prints the reference export's IFC schema and Revit release from its header. Counts only for held-out projects.

## 3. Measured

Measure 43714011584, 84 models, the spent project listed per element.

**Row 1: reach.** The 103 are pairs, one for each door or window of Revit 2015 to 2023 whose map holds a -1 and each bound unset form of its family; a form is counted once for every element whose family holds it, not once, and the probe does not count distinct forms. The pairs are in three projects. 54 resolve and 49 do not; every one of the 49 is an ambiguous caption, and none is a parameter whose caption or value was not read. They are the spent project's: 4 of its 53 resolve. The other two projects' 50 all resolve (Warmond's 30, BIM Whale's 20); Frigento has none.

**Rows 2 and 3: the cross-tab.** Doors and windows whose map holds geometry mapped to no material, by the family's category, Revit's `<Unnamed>` written / not written:

| Project | Category | Written | Not written |
| --- | --- | ---: | ---: |
| Spent Revit 2018 (IFC2X3), windows | an unset form effectively shown | 8 | 11 |
| | an unset form unresolved, none shown | 44 | 5 |
| Frigento (Revit 2020, IFC4), doors | an unset form effectively shown | 0 | 22 |
| Warmond (Revit 2021, IFC4 through a preprocessor, counted apart), windows | an unset form effectively shown | 0 | 54 |
| Warmond (counted apart), doors | an unset form effectively shown | 0 | 14 |
| BIM Whale (Revit 2021, IFC2X3, counted apart), doors | an unset form effectively shown | 0 | 10 |
| BIM Whale (counted apart), windows | no unset form, a nested instance | 0 | 263 |

The spent project's 68 windows by every fact read:

| Windows | Facts | Written | Not written |
| ---: | --- | ---: | ---: |
| 15 | one unset form, unbound, effectively shown; one void; one nested instance | 8 | 7 |
| 4 | two unset forms, one bound and resolved by its parameter (to shown on 3 windows, to hidden on 1), the other unbound and shown; no void; three nested instances | 0 | 4 |
| 49 | a bound unset form whose caption is ambiguous; one void; one nested instance | 44 | 5 |

**Row 4: the reference export.** Doors and windows of Revit 2015 to 2023 with a -1 in the map have a reference export from the same Revit release as the save in all four projects with any: the spent project's Revit 2018 (IFC2X3), a Revit 2020 (IFC4), a Revit 2021 (IFC4) and a Revit 2021 (IFC2X3). `<Unnamed>` is written on 52 of 68 of the first and on none of 363 of the other three (Frigento, Warmond, BIM Whale), so the schema does not separate (IFC2X3 is on both sides) and the exporter's release does only as the save's does. Of the 363, only Frigento's 22 doors come from an unmodified Revit IFC4 export; Warmond's passed through a preprocessor and BIM Whale's is IFC2X3, and both are counted apart. Two other pre-2024 saves with a reference export, a Revit 2020 save exported by Revit 2021 (IFC2X3) and a Revit 2023 save exported by Revit 2026 (IFC4), hold no door or window with a -1.

**Records held in several partitions.** A family's document can hold a record in more than one partition (`family_records_in` returns every copy, oldest first, and a reader that keeps the first or counts each would count a stale copy). Added after review of the first run, not a registered row: the second run's probe keeps each record's last copy and counts the records held more than once and those whose copies differ. Over the 84 models, 12,220 records of families' documents are read (Revit 2015 to 2027: 525, 380, 346, 1,064, 588, 573, 1,075, 410, 419, 3,279, 2,018, 921 and 622 by release) and none is held in more than one partition, so none has copies that differ. The first run's counts therefore stand, and the second reproduces its 8,272 per-element `wbound` lines byte for byte and every cross-tab cell. This holds for these 84 models only; the Revit 2024 to 2027 family-forms reader (`native_family_forms`) is not changed by it.

## 4. The registered rows

Registered on #1603 before the run (2026-11-15, comment 6893649932), read from Measure 43711618548 and, for the same figures, 43714011584.

1. **Reach.** Reported: 54 of 103 pairs of an element and a bound unset form resolve, the 49 that do not all on the spent project and all by an ambiguous caption (RE-483's probe leaves such a form unresolved rather than choose a parameter).
2. **The spent project.** Not met: no rule is right on all 68. The 15 windows of the first row share every fact read and split 8 and 7, so none of the facts read of the host family's forms, whatever the parameters say, separates them.
3. **Other pre-2024 projects.** Reported: all 50 pairs elsewhere resolve, and the cross-tab is RE-482's: an unset form shown on 100, a nested instance and none on 263, and `<Unnamed>` written on none of the 363 (only Frigento's 22 are measured against an unmodified Revit IFC4 export).
4. **The exporter.** Reported: the release of the exporter and the save coincide in every project with an unset map, so a release effect cannot be told from the save's with these files.
5. **Nothing moves.** Met: no `src/` change.

## 5. What this decides and does not

- **Decided.** A `Visible` bound to a family parameter, read from the parameter's value, does not decide `<Unnamed>` before 2024. Of the 68 windows of the spent project 15 hold the same facts read of the host family's forms and differ in the result, so the cause is not among the facts read. The `<Unnamed>` rule of RE-204 stays unapplied before 2024 (RE-481).
- **Decided.** Of the 103 pairs of a door or window and a bound unset form of its family in Revit 2015 to 2023, 49 have a caption that names two parameters, all on the spent project; a reader that keys parameters by caption (as `instances_without_body` does) does not know their value there. That is a limit of keying by caption, not of this question.
- **Not decided: what writes `<Unnamed>` before 2024.** Not among the facts read of the host family's forms (RE-482, RE-483). Candidates not read: the nested families' own forms (every window of the spent project and of BIM Whale nests an instance), the forms' subcategory and object-style materials (RE-481 names the object-style material), and the instance's or type's own facts. Whether the 15 windows of the first row share a type or a family is not in these counts; it says whether the cause is the type's or the instance's (RE-485 reads it).
- **Not decided: whether the exporter's release does.** Not measured: Revit 2015 to 2018 exports other than the spent project's, and any pre-2024 save exported by a different release with an unset map.

## 6. Reproduce

```bash
# the probe at 04c2dfd9d977, the samples and every held-out project (the held-out counts need the scrub of this change, which --ref main holds only once it lands)
gh workflow run measure.yml --ref main -f ref=04c2dfd9d97737a99e52cbe9c6f3d41198013664 -f base=none -f samples=true -f holdout=true \
  -f scorers=none -f probe=probe_1603_unnamed_bound_before_2024 -f probe_scorer=window_door_unnamed_bound_vs_ifc
# each model's probe.txt holds the `wbound` lines and the `bound visible values:` and `family record copies:` lines; the spent project's probe_score.txt lists every counted element; a held-out model's counts.txt holds the `unnamed bound before 2024:`, `reference export:`, `bound visible values:` and `family record copies:` lines
```

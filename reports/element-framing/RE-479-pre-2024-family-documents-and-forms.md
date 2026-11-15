# RE-479: 367 of the 429 families of the doors and windows of Revit 2015 to 2023 projects give decoded forms, as 330 of 382 do from Revit 2024, and the forms hold their material in the older id shape

**Date:** 2026-11-15
**Issues:** #1603 (Revit 2015 to 2023: the 70 windows of the spent Revit 2018 project lack Revit's materials); follows RE-478 (the map to read is the drawn symbol's; the rules it lacks are the family forms' and the partly unset types' material parameters), RE-473 (the type material map is on every 2015 to 2023 save, in the older id shape), RE-204 to RE-206 (when `<Unnamed>` is written, and the materials only hidden forms hold), RE-163 (a family's document is a keyed block named by its `Family` record's key), RE-280 (the two ElementId shapes).
**Artefacts:** the spent Revit 2018 project, the 18 held-out projects of Measure (counts only; one of them is the held-out save of the spent project), and the 65 reference models and Autodesk samples of Revit 2015 to 2027, 84 models in all, with no export changed.
**Probe:** `probe_1603_family_forms_pre2024` at `fcf37a5f1864` (Measure 43701506486), archived under `refs/archive/2026-11-15/cited/`. No scorer: the probe prints counts, and the held-out scrub keeps its fixed-wording lines.
**Status:** positive for the forms, negative for row 1 as registered. On every release from 2015 to 2027 the forms of the families that give any decode, with `m_materialId`, `m_cutting` and the stored `Visible` present, the material in the older id shape on every save before 2024 and in the 2024 shape on every save from it, and no file holding both. The share of the families that give a decoded form or nested instance (what the probe counts as "with a document": a family whose document is found but holds none, and a family whose document is not found, are not told apart) is under the registered 90% on seven of the nine releases before 2024 (2015, 2016, 2017, 2019, 2021, 2022, 2023), on 2024 (81.0%) and on 2027 (89.1%), and the number of families that give none is the same on every save of the same project: no release is worse than another (367 of 429 before 2024, 330 of 382 from it). Every project with doors and windows to score against Revit's export (the spent project and the 17 other held-out ones) gives a decode for every one of its families.

## 1. The question

RE-478 found that the drawn symbol's type map is Revit's material set on 414 of the 846 doors and windows of Revit 2015 to 2023 (as RE-478 counts them now; it first counted 558 of 992, which included 146 of Revit 2024 to 2027), and that the rules which would bring the other 387 in line are the ones `native_family_forms` reads on Revit 2023 to 2027 (RE-204 to RE-206) and `attach_partly_unset_materials` applies. `native_family_forms` takes an ElementId in the 2024 shape only. Does a save before 2024 hold the families' own documents, do their forms decode, and in which id shape do they hold their materials and their `Visible`?

## 2. What the probe counts

For the doors and windows the export writes, the probe takes each instance's master symbol and that symbol's family, reads the family's document through `family_records_in` over releases 2015 to 2027, and counts, per release and per form class (`ExtrusionElem`, `RevolutionElem`, `SweepElem`, `BlendElem`, `SweptBlendElem`, `FreeFormElement`, `Text3dElem`): the forms, their `m_materialId` in the 2015 to 2023 shape (`{"m_id":{"m_id":n}}`) and in the 2024 shape (`{"m_id":{"m_id64":n}}`), the forms mapped to no material (-1, "unset"), forms with no material at all ("none"), `m_cutting`, the stored `Visible` (a `ParamValueSetInt` entry of the form's parameter set for parameter -1006205) and the `Visible` bound to a family parameter (a `FamilyParametrizedElemParamsCell` entry for the same element property). Counts only.

A family is counted "with a document" when `family_records_in` returns at least one decoded record of the eight classes asked for (the seven form classes and `FamilyInstance`): a family whose document is found but holds none of them, and a family whose document is not found, are not told apart.

## 3. Measured

**Row 1: documents.** Families that give a decoded form or nested instance ("with a document" in the probe's line), over all 84 models, by release (pooled; the number of models in brackets):

| Release | Families | Giving a decode | Share |
| --- | ---: | ---: | ---: |
| 2015 (6) | 52 | 40 | 76.9% |
| 2016 (3) | 36 | 31 | 86.1% |
| 2017 (3) | 36 | 31 | 86.1% |
| 2018 (5) | 58 | 53 | 91.4% |
| 2019 (6) | 64 | 49 | 76.6% |
| 2020 (5) | 54 | 49 | 90.7% |
| 2021 (5) | 47 | 42 | 89.4% |
| 2022 (5) | 40 | 35 | 87.5% |
| 2023 (6) | 42 | 37 | 88.1% |
| 2024 (13) | 195 | 158 | 81.0% |
| 2025 (12) | 86 | 81 | 94.2% |
| 2026 (8) | 55 | 50 | 90.9% |
| 2027 (7) | 46 | 41 | 89.1% |

Before 2024: 367 of 429. From 2024: 330 of 382. 65 of the 84 models give a decode for every family; the other 19 are the structural samples' saves (rst basic on each of its 13 saves, 2015 to 2027; rst advanced on 2015 and 2019) and four 2024 Snowdon Towers samples (electrical, HVAC, plumbing, structural). The spent project (11 of 11) and all 18 held-out projects give a decode for every family. Each sample project that has a save before 2024 and a save from 2024 gives the same count on both: architectural basic and advanced (7 of 7 and 9 of 9 on every save, 2015 to 2027), MEP basic and advanced (3 of 3), structural basic (15 of 20 on every save from 2016 to 2027; its 2015 save lists 8 families, 6 giving a decode, a smaller list of doors and windows that is not explained here), structural advanced (12 of 22 on 2015 and on 2019).

**Row 2: the forms decode.** Over the models of each release (forms; material in the 2015 to 2023 shape; in the 2024 shape; unset; none; cutting):

| Release | Forms | 2015 to 2023 shape | 2024 shape | Unset | None | Cutting |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 2015 | 106 | 101 | 0 | 5 | 0 | 1 |
| 2016 | 82 | 79 | 0 | 3 | 0 | 1 |
| 2017 | 82 | 79 | 0 | 3 | 0 | 1 |
| 2018 | 162 | 115 | 0 | 47 | 0 | 13 |
| 2019 | 118 | 111 | 0 | 7 | 0 | 1 |
| 2020 | 139 | 128 | 0 | 11 | 0 | 5 |
| 2021 | 169 | 108 | 0 | 61 | 0 | 34 |
| 2022 | 106 | 103 | 0 | 3 | 0 | 1 |
| 2023 | 106 | 101 | 0 | 5 | 0 | 1 |
| 2024 | 582 | 0 | 315 | 267 | 0 | 22 |
| 2025 | 451 | 0 | 345 | 106 | 0 | 3 |
| 2026 | 146 | 0 | 124 | 22 | 0 | 11 |
| 2027 | 126 | 0 | 119 | 7 | 0 | 1 |

Every form has a material in one shape or is mapped to none (-1): no form lacks the member, and no file holds forms in both shapes. By class, before 2024 (1,070 forms): 863 `ExtrusionElem` (748 with a material, 115 unset), 202 `SweepElem` (175, 27), 3 `BlendElem` (all unset) and 2 `RevolutionElem` (both with a material); from 2024 (1,305 forms): 811 extrusions, 232 sweeps, 26 blends (all unset), 25 revolutions, 208 `FreeFormElement` (all with a material), 2 `SweptBlendElem` and 1 `Text3dElem`. No pre-2024 save of these projects holds a free-form, swept-blend or text form. Every sample project with several saves gives the same forms per class (count, unset, cutting, stored, bound) on every one of them, only the id shape differs, with two exceptions in the structural basic project: its 2015 save (above), and its 2016 save, which reads one form as a sweep where its later saves read an extrusion (13 and 8 against 14 and 7, 21 forms on each; not explained here). The spent project and its held-out save: 40 forms, 22 of them unset (16 of 28 extrusions, 6 of 12 sweeps), 6 cutting.

**Row 3: Visible.** The stored `Visible` is on every form of every release (106 of 106 on 2015, 82 of 82 on 2016, and so on to 126 of 126 on 2027). The probe takes the parameter's id in either shape and does not count which, so that the older shape holds it on a save before 2024 is expected (a file holds one shape, above) and not counted. The binding to a family parameter, by release: 2015 2, 2016 2, 2017 2, 2018 12, 2019 2, 2020 2, 2021 6, 2022 2, 2023 2; 2024 32, 2025 223, 2026 13, 2027 10.

**Row 4: nothing moves.** The branch changes the probe and the held-out scrub only (`git diff --stat main...`: 2 files). No `src/` change, no export.

## 4. The registered rows

Registered on #1603 before the run (2026-11-15, comment 6892618844), read from Measure 43701506486.

1. **Documents.** Not met as registered: the releases 2015, 2016, 2017, 2019, 2021, 2022, 2023, 2024 (81.0%) and 2027 (89.1%) are under 90%. Reported above. What the row was written to catch, a release before 2024 whose documents are not found where the 2024 saves' are, is absent: every project with saves on both sides gives the same count on each, and the projects the doors and windows are scored on give a decode for every family. The registered consequence, that the release's family documents are found first, is not followed, and the departure is this: the count is of families that give a decoded record, so it cannot tell a document that is absent from one that holds no solid form; the shortfall is the same on 2023 and on its 2024 save, where the reader reads the documents today; and the next change fails closed where nothing decodes. Which of absent and empty it is stays open (section 5).
2. **Forms decode.** Met on all thirteen releases: forms per class, the material in the 2015 to 2023 shape before 2024 and in the 2024 shape from it, no file with both.
3. **Visible.** Met as to found (the stored `Visible` on every form of every release, the binding where a family has one); the shape it is found in is not counted apart.
4. **Nothing moves.** Met.

## 5. What this decides and does not

- **Decided.** Where a family gives a decode, its forms on 2015 to 2023 saves are in the structure the 2024 reader takes, and differ from it in the id shape of `m_materialId` (measured), and presumably in the ids around it (the `Visible` lookups took either shape). `native_family_forms` reading either shape on those releases was the next change: where a family's forms decode it would give the unset-geometry and hidden-form rules, and where none decodes it would give nothing. RE-481 scored those rules on the older releases and they do not carry over, so no export reads the forms before 2024.
- **Not decided: whether the rules bring the 387 elements in line.** That the forms are readable does not say that RE-204 to RE-206 decide the 299 windows and doors that differ only in whether `<Unnamed>` is written, or the 79 whose map holds names Revit's set does not (RE-478 section 5). RE-481 answered it: they do not.
- **Not decided: the families that give no decode.** Which families of the structural samples and the four Snowdon samples they are, and whether their document is absent from the file (a system or in-place family, or a document under another key) or present with no solid form, is not measured; the same number of families give none on every save of the same project, from 2015 to 2027, so it is not a property of an older release.
- **Not decided: two saves of the structural basic project:** the 2015 save lists 8 families where its later saves list 20, and the 2016 save reads one form as a sweep where the later saves read an extrusion.
- **Not decided: the nine windows' `LKS_VEN_MADERA`** (RE-478), in no record the probe reads.

## 6. Reproduce

```bash
# the probe at fcf37a5f1864, the samples and every held-out project (the held-out counts need the scrub of this change; main holds it once it lands)
gh workflow run measure.yml --ref main -f ref=fcf37a5f18645cf874d18ce0afbc8fd84af9d49a -f base=none -f samples=true -f holdout=true \
  -f scorers=none -f probe=probe_1603_family_forms_pre2024
# each model's probe.txt (the held-out projects' counts.txt) holds the `family forms pre2024:` lines
```

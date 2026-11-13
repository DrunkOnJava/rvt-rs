# RE-459: the episode rule for element GlobalIds against Revit's exports of Revit 2015 to 2023 files

**Date:** 2026-11-13
**Issues:** #1929 (the rule runs on every 28-byte ElemTable, and RE-208 measured Revit 2015 as not episode-derived), #878 (Revit 2015's elements' GlobalIds); follows RE-48 (the rule), RE-108 (it on Revit 2023), RE-208 (a Revit 2015 export it does not rebuild), RE-220 (the GlobalIds a file stores) and RE-326
**Artefacts:** Autodesk's `rac_basic_sample_project` saved by Revit 2018 and the public Revit 2018 export of it that RE-326 describes (sha256 `1114ce918f64885bd87ec3bc670c4c543a90898d1bf7e701fb8fbf95e5b5ea35`, 9,129,271 bytes, a forum attachment with no licence stated, used locally and never committed); the held-out Revit 2018 to 2021 projects of `research/holdout-archive-models.tsv`, each with the same release's own IFC export, and the spent Vitoria "Before" model beside them; BIMcollab's Revit 2015 example (RE-198, RE-208)
**Scorer:** `tools/re/global_ids_vs_ifc.py` (elements paired by Tag, openings left out, storeys by name, spaces by GlobalId)
**Runs:** Measure [43477851375](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43477851375) (every flag, the held-out models as counts only), [43486849702](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43486849702) (`-f keep_ifc=true -f samples=true -f sample_years="2018 2018"`, the 2018 save's export scored locally against the public one), [43485868407](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43485868407) (`probe_878_stored_guids` on the spent Vitoria model and BIMcollab's)
**Status:** positive for the rule on 28-byte tables of 2018 to 2023, negative for the files that do not follow it. The rule is kept on every 28-byte table and not gated to 2023: it is Revit's on 2020, 2021 and 2023 files and on the 2018 sample, and gating would take those away. Two Revit 2018 projects and BIMcollab's 2015 example are not explained by it, and RE-208's finding for 2015 is a finding about that file.

## 1. The question

`revit_global_ids` picks the episode number's offset from the ElemTable record length alone: 40 bytes (Revit 2024 and later) or 28 (every release through 2023). The rule RE-48 measured on 2024 and RE-108 on 2023 therefore also runs on 2015 to 2022. RE-208 measured one Revit 2015 export whose element GlobalIds are not episodes, and the audit of the research the code cites (#1929) read the other 28-byte releases as unmeasured. Do the elements of a Revit 2015 to 2022 file take Revit's GlobalId from the rule, and should it be gated to 2023?

## 2. What is measured, by release

| Release | File | Elements equal to Revit's | Storeys | Project, site, building |
|---|---|---|---|---|
| 2015 | BIMcollab structural example (RE-208) | 0 of 241 | 5 of 5 (stored values, RE-220) | 3 of 3 (stored values) |
| 2018 | `rac_basic_sample_project`, against the public export | 434 of 435 (the other is a door); 14 of 14 rooms | 0 of 6 | 0 of 3 |
| 2018 | Vitoria "Before", spent | 0 of 1,314 | 0 of 7 | 0 of 3 |
| 2018 | Vitoria "Project", held out (counts only) | 0 of 1,441 | 0 of 7 | 0 of 3 |
| 2020 | Frigento, held out | 145 of 151 | 0 of 4 | 0 of 3 |
| 2021 | Gdynia, held out | 181 of 181 | 0 of 5 | 0 of 3 |
| 2021 | BIM Whale AdvancedProject, held out | 1,696 of 1,698 | 0 of 16 | 0 of 3 |
| 2021 | Warmond, held out, through an IFC preprocessor (counted apart) | 447 of 531 | 6 of 9 | 3 of 3 |
| 2023 | `Exemplo_data`, `modelo_bim` (RE-108) | 45 of 45 and 36 of 37 | 2 of 2 and 4 of 4 | not scored |

- **Elements.** On the four files whose export Revit wrote and nothing rewrote, the rule gives Revit's GlobalId to 2,456 of 2,465 elements both exports hold (99.6%): 434 of 435 on the 2018 sample, 145 of 151 on Frigento, 181 of 181 on Gdynia and 1,696 of 1,698 on BIM Whale. Spaces follow: 14 of 14 rooms of the 2018 sample, and 18 of 19, 6 of 6 and (Warmond) 179 of 185 on the held-out models. The 14 rooms of the sample also have the same GlobalIds on every save of the project from 2015 to 2023, equal to the public 2018 export's (RE-326), and RE-333 pairs 116 of 116 spaces of the 2020 save of `rac_advanced_sample_project` with an IFC2X3 export of it.
- **A release is not what fails.** Both Revit 2018 projects of the BIM-Speed set read 0 of 1,314 and 0 of 1,441 while the 2018 sample reads 434 of 435, so the rule does not fail on Revit 2018 as a release. The same holds for 2015: RE-208's example is one file whose ids are not episodes.
- **Storeys, project, site and building are not equal on a file before 2023.** None of the files above that is not preprocessed reads one equal, except BIMcollab's example, whose values the file stores and RE-220 reads as stored (5 of 5 storeys, 3 of 3). RE-108 reads 6 of 6 storeys of the two Revit 2023 files.

## 3. What this rules out

- **Gating the rule to 2023 is wrong.** It would remove ids that are Revit's on a Revit 2020 and two Revit 2021 files (2,022 of the 2,030 elements paired there, counts only) and on the 2018 sample, and the fallback is an id derived from the document, which changes with every save (#1854).
- **"2016 to 2022 are unmeasured" was wrong:** 2018, 2020 and 2021 are measured; 2016, 2017, 2019 and 2022 have no Revit export of their elements here, and the rooms of the 2018 lineage read the same on every save from 2015 to 2023 (RE-326).

## 4. What is not explained

- **Vitoria's two Revit 2018 projects (0 of 1,314 and 0 of 1,441).** RESULT_PENDING
- **The storeys of a file made before 2023 (0 of 45 on the six files above that are neither preprocessed nor store them).** RESULT_PENDING

## 5. Not changed

- Nothing is exported differently. `revit_global_ids` stays as it is on every 28-byte table, and its documentation says what is measured where.

## 6. Reproduce

```bash
python3 .claude/scripts/measure-dispatch.py --ref <main commit> --base none -f keep_ifc=true -f samples=true -f sample_years="2018 2018" -f models="sample-2018-rac_basic_sample_project" -f scorers=none
python3 tools/re/global_ids_vs_ifc.py <the run's model.ifc> rac_basic_sample_project.ifc
python3 .claude/scripts/measure-dispatch.py --ref <main commit> --base none -f holdout=true -f models="spent-* holdout-*"   # the held-out models read as counts
```

The public Revit 2018 export is the attachment `rac_basic_sample_project.ifc` of the IFC for Revit wiki's "Exporting Zones" discussion on SourceForge (`ifcexporter`), fetched by hand into an empty folder and checked against the sha256 above.

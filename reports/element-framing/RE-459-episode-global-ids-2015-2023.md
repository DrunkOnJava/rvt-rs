# RE-459: the GlobalIds of a project, building, site, storey and element against Revit's exports of Revit 2015 to 2027 files

**Date:** 2026-11-13
**Issues:** #1929 (the episode rule runs on every 28-byte ElemTable, and RE-208 measured Revit 2015 as not episode-derived), #878 (the GlobalIds of Revit 2015 to 2022 files); follows RE-48 (the episode rule), RE-108 (it on Revit 2023), RE-208 (a Revit 2015 export it does not rebuild), RE-220 (the GlobalIds a file stores, and the project-level base it did not find) and RE-326
**Artefacts:** Autodesk's `rac_basic_sample_project` saved by Revit 2018 and the public Revit 2018 export of it that RE-326 describes (sha256 `1114ce918f64885bd87ec3bc670c4c543a90898d1bf7e701fb8fbf95e5b5ea35`, 9,129,271 bytes, a forum attachment with no licence stated, used locally and never committed); the 67 models of Measure that have a Revit export (Core Interior, the RE1 four, the 12 BIM4LCA models, the 24 Revit 2025 models, rac_advanced 2015 to 2020 against its export that carries no licence, BIMcollab's Revit 2015 example, the spent Vitoria "Before" and the 18 held-out files, the last four groups as counts only)
**Probes and scorers:** `probes/examples/probe_re459_history_terminators.rs` (the history's entry bytes, the project's GUID in its header, the oldest and newest episode, every GlobalId the file stores, the Levels), `tools/re/history_base_vs_ifc.py` (Revit's ids against them), `tools/re/global_ids_vs_ifc.py` (rvt-rs's ids against Revit's)
**Runs:** Measure [43477851375](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43477851375) (main, every flag), [43486849702](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43486849702) (the 2018 sample's export, kept), [43485868407](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43485868407) (`probe_878_stored_guids`), [43487391258](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43487391258) (the episode census of the spent Vitoria), [43488971062](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43488971062) (the history's entry bytes), [43490409752](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43490409752) (the header GUID on 121 models; the scorer I wrote refused Measure's `--list` argument, so nothing was scored), [43493324842](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43493324842) (the header GUID against Revit's exports of 67 models), [43496098202](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43496098202) (the GlobalIds the files store), and the unfiltered run of the writer change [43498066087](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43498066087)
**Status:** positive for three findings and negative for four. Positive: the project's GUID in the header of `Global/History` is the base of Revit's IfcProject, IfcBuilding and one IfcSite on 63 of the 64 files of 2017 to 2027 whose Revit export is of their own project; it is the base of the storeys of 2017 to 2022 (69 of 69 on 11 files); and the episode rule is Revit's for elements on every file it could read once the history parse stopped refusing four entries. Negative: the base is not the storeys' on 2023 and later (0 of 229, where each Level's own episode is); a file that stores its ids is written as stored (BIMcollab's Revit 2015 example); Revit 2015 and 2016 have no file to measure the base rule on; and one held-out Revit 2018 file follows no rule here.

## 1. The questions

1. Why does `revit_global_ids` give none of the 1,314 elements of the Revit 2018 Vitoria "Before" model Revit's GlobalId, while the same rule gives 434 of 435 on Autodesk's 2018 sample?
2. RE-220 §1 found no source for the GlobalIds of a project, building, site and storey before Revit 2023 (0 of 45 storeys, 0 of 3 each), and no GUID in the file that RE-220's search recognised as their base. Where is it?
3. The audit of the research the code cites (#1929) read the episode rule as unmeasured on 2016 to 2022. Is it gated to 2023?

## 2. The history parse

RE-48 reads `Global/History` as a count, a `u32`-counted table, the count again and that many 17-byte entries, each a GUID and the byte `0x28`, and `EpisodeHistory::parse` returned `None` when any entry ended in another byte. On the spent Vitoria "Before" (Revit 2018, 1,127 entries) four entries (the 12th, 24th, 44th and 47th) end in `0x1e`; the others, 1,123, in `0x28` (probe run 43488971062: `terminators 1e=4 28=1123`). Each of the four is a 16-byte GUID like its neighbours. So `parse` refused the history, `revit_global_ids` returned an empty map and every element got a GlobalId derived from the document: that is the 0 of 1,314. (The explanation tested first, that this file stores its ids, was wrong: no 22-character string it stores is tied to any GlobalId, run 43485868407.)

The episode census (run 43487391258) shows Revit's ids on that file are episode-derived all the same: of its 1,321 elements with a numeric Tag, 1,298 have a GlobalId whose prefix (the ElementId XORed out) is an entry of the history, and for 1,233 of them the episode number at `+8` of the 28-byte record, counted from the last entry, names that entry, as RE-48 reads it. `EpisodeHistory::parse` now reads an entry whatever byte closes it, and keeps the refusals that matter (counts that disagree, a count past the stream, a table that overflows), with failure-first tests of those. With the history read, the unfiltered run of the writer change (43498066087, head `e5516b73`) gives the spent Vitoria 1,297 of its 1,314 elements Revit's GlobalId, from none, and its 73 rooms 73 of 73, from none; the held-out Revit 2018 Vitoria project, whose project-level ids no rule gives, gets 1,420 of its 1,441 elements and 73 of 73 rooms, also from none. The 65 elements whose episode lies before the `0x1e` entries (the census's other 65) read Revit's id as well: an entry closed by `0x1e` is an episode like the others, and counting it is what their `+8` number does.

## 3. The base: the project's GUID in the header

The first `0x100` bytes of `Global/History` hold one GUID four times: at `0x16`, `0x26` and `0x36`, then 16 zero bytes, then at `0x56`, which ends where the table's count at `0x66` begins. Measured on every file the probe read:

- **It is always four equal copies.** 268 of 268 copies equal their first on the 67 models that have a Revit export (the held-out ones as counts), and on all 121 probe outputs that are kept in full, 19 family files included.
- **It is not an episode.** Compared with the oldest and the newest entry of the history, it is neither on any of the 67 models (0 of 67 and 0 of 67). This is why RE-220's search, which looked among the history's entries and in the file's text, did not find it.
- **It is a project's identity, kept across saves and releases.** The 121 files hold 35 distinct values, and files saved from one another share one: the 11 releases of `rac_basic` from 2017 to 2027, the 11 of `rst_basic`, the 7 of `rac_advanced` from 2021 to 2027, the 24 Revit 2025 models, the six BIM4LCA housing models and the six office models, the 12 `rac_basic_sample_family` files. It is not constant for ever: the 2015 and 2016 saves of `rac_advanced` hold another than 2017 to 2020 do.

Revit's exporter gives the project, the building and the site the GlobalIds of the fake ElementIds -16, -15 and -14 (revit-ifc `GUIDUtil.ProjectLevelGUIDType`), and RE-220 found that each is one GUID with the fake id XORed into its last 32 bits. The header GUID is that GUID:

| Revit's export | Is the header GUID with this XORed into its last 32 bits | Models |
|---|---|---|
| IfcProject | -16 | 63 of 67 |
| IfcBuilding | -15 | 63 of 67 |
| one IfcSite (of 106 in the exports: Revit writes one for each topography too, 5 on the BIM4LCA housing, 18 on one held-out file) | -14 | 63 of 67 |
| IfcBuildingStorey | the Level's ElementId, on 2017 to 2022 | 69 of 69 storeys on 11 files |

The 63 models are every file of 2017 to 2027 with a Revit export of its own project but one: Core Interior, the RE1 four, the 12 BIM4LCA models, the 24 Revit 2025 models, 17 of the 18 held-out files, rac_advanced 2017 to 2020 and the spent Vitoria. The 2018 sample is the same on its project and building (the site there is a topography element's) and on its 6 storeys, checked on the public export. The four models that do not follow it:

- **rac_advanced 2015 and 2016.** Their header holds another GUID than 2017 to 2020's, and the export they are scored against is of the later project, which 2017 to 2020 match (1 of 1, 5 of 5 storeys). That is a different project's export, not a measurement on these two files.
- **BIMcollab's Revit 2015 example.** The file stores its project, building, site and 5 storeys as strings (8 of 8), and Revit writes what is stored; 269 of the 292 products of its export equal a stored string but for the first character (RE-208's 229 of 241 on its elements).
- **The held-out Vitoria project (Revit 2018).** Nothing of Revit's is the header GUID's, a stored string, or (on main) an episode's: 0 of 1 on each of the three, 0 of 7 storeys, 0 of 2,907 of the elements as stored strings. It is one file of the 64 and stays unexplained.

**The storeys follow the base only before 2023.** On the 12 files of 2017 to 2022 that have a Revit export of their own project, the 69 storeys of Revit's on 11 are the header GUID with the Level's ElementId XORed in (rac_advanced 2017 to 2020, the spent Vitoria, and six held-out files of 2020 to 2022), and on the twelfth it is 0 of 7. On every one of the 52 files of 2023 and later it is 0 of 229, and there each storey is the GUID of the Level's own episode with its ElementId XORed in, as RE-48 and RE-108 read it (Core Interior 15 of 15, the Revit 2025 models 4 of 4, the BIM4LCA office 10 of 10, the held-out files of 2023 to 2027 all). The oldest episode's GUID gives one storey of 51 of those 52 (the Level made in the first episode, none on Core Interior).

## 4. The elements

On the four files whose export Revit wrote and nothing rewrote, the episode rule gives Revit's GlobalId to 2,456 of 2,465 elements both exports hold (99.6%): 434 of 435 on the 2018 sample, 145 of 151 on Frigento (2020), 181 of 181 on Gdynia (2021) and 1,696 of 1,698 on BIM Whale (2021). Spaces follow: 14 of 14 rooms of the 2018 sample, and 18 of 19, 6 of 6 and (Warmond) 179 of 185 on the held-out models. The 14 rooms of the sample also have the same GlobalIds on every save of the project from 2015 to 2023, equal to the public 2018 export's (RE-326), and RE-333 pairs 116 of 116 spaces of the 2020 save of `rac_advanced_sample_project` with an IFC2X3 export of it. RE-108 measures the 2023 files (45 of 45 and 36 of 37).

- **A release is not what fails.** The two Revit 2018 projects of the BIM-Speed set read 0 of 1,314 and 0 of 1,441 while the 2018 sample reads 434 of 435. For the first the cause is the history parse (§2). The second is the held-out file of §3 that no rule explains; whether the parse change moves its elements is counted in §5.
- **Gating the rule to 2023 is wrong.** It would remove ids that are Revit's on a Revit 2020 and two Revit 2021 files (2,022 of the 2,030 elements paired there, counts only) and on the 2018 sample, and the fallback is an id derived from the document (#1854).
- **A file that stores ids.** The file stores the project, building and site ids on 14 of 67 models (the 12 BIM4LCA models, one held-out Revit 2021 file and BIMcollab's) and the storeys' on 2 (that held-out file and BIMcollab's). On the 13 where the base rule is Revit's too, the stored string and the base-derived id are the same GlobalId, so stored first, then the base, then the episode is what the files show. A lead that is not part of this rule: on the held-out Revit 2021 file 626 of the 1,149 products of Revit's export are stored strings (725 of 107,532 over all 67 models), and the episode rule gives it 447 of the 531 elements it pairs, with 84 that differ; whether the 84 are the stored ones is for a later probe.

## 5. What is not explained

- **The held-out Revit 2018 Vitoria project (§3)**: one file, nothing found that gives its ids.
- **Revit 2015 and 2016**: no file here has a Revit export of its own project that stores none, so the base rule is not claimed for them.
- **The 23 elements of the spent Vitoria whose GlobalId is in no entry of its history** (1,321 less 1,298).
- **The 17 elements of the spent Vitoria and the 21 of the held-out Vitoria project that still differ** (Revit's id is in no entry of the history for 23 of the 1,321 of the first, by the census).

## 6. What the exporter does

`revit_model_global_ids` takes, for the project, the building and the site, the id the file stores, else (Revit 2017 and later) the project's GUID with -16, -15 or -14 XORed in; for a storey the id the file stores, else (Revit 2017 to 2022) the project's GUID with the Level's ElementId XORed in, else the Level's episode (2023 and later, and 2015 and 2016). The elements are the episode rule as before, with the history read whatever byte closes an entry. A project's GUID that is missing, differs between its four copies or is zero gives none, and the writer derives the id as it did. Writing the project-level ids this way changes the GlobalIds of every file of 2017 to 2027 that stores none, so it ships once, in the release that carries any other GlobalId move.

## 7. Reproduce

```bash
python3 .claude/scripts/measure-dispatch.py --ref <the pull request's head commit> --base none --scorers-ref <its branch> -f nordic=true -f r2025=true -f bimcollab=true -f racadv=true -f holdout=true -f probe=probe_re459_history_terminators -f probe_scorer=history_base_vs_ifc -f scorers=none -f models="core-interior nordic-* r2025-* re1-* spent-* holdout-* bimcollab-* racadv-*"
python3 .claude/scripts/measure-dispatch.py --ref <the same commit> --base none -f samples=true -f families=true -f probe=probe_re459_history_terminators   # the header GUID of every sample and family
uv run --no-project --with ifcopenshell python3 tools/re/history_base_vs_ifc.py <probe output> rac_basic_sample_project.ifc --list   # the 2018 sample against its public export
```

The public Revit 2018 export is the attachment `rac_basic_sample_project.ifc` of the IFC for Revit wiki's "Exporting Zones" discussion on SourceForge (`ifcexporter`), fetched by hand into an empty folder and checked against the sha256 above. The probe prints the GUIDs of a file and its Levels' names and ElementIds; a held-out model's lines never leave the runner, because the scrub keeps only the scorer's counts.

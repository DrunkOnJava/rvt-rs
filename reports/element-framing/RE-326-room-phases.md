# RE-326: a room's phase is `RoomElem.m_phaseId`, and a Revit export holds rooms that are not in the last phase

**Date:** 2026-11-04
**Issue:** the "Rooms and phases" item of #1194; follows RE-184 §5 (#328).
**Artefacts:** 12 models with rooms and a Revit export (Core Interior, RE1 Architecture, the BIM4LCA office as built and at building permit, its two terrain variants, six R2025 room models: 621 rooms), Autodesk's sample projects of every release 2015 to 2027 (no Revit export in the corpus), and one public Revit 2018 export of Autodesk's `rac_basic_sample_project` (below). Held-out models were not read.
**Probes:** `probes/examples/probe_re326_room_phases.rs` and `probes/examples/probe_re326_phase_census.rs`; scorers `tools/re/room_phases_vs_ifc.py` and `tools/re/phase_census_vs_ifc.py`.
**Status:** positive for where a room's phase is stored and for the default rule; unmeasured against a Revit export made with the default setup (§9 adds a fallback for a project whose last phase holds no room, a choice by Griffin, and not a measurement). A room's phase is `RoomElem.m_phaseId`. Revit's exporter leaves out a room whose phase is not the export phase, the last unless the export setup or a view names another, and rvt-rs does too, with `rvt-ifc --phase <name|id>` naming the export phase (Griffin, 2026-11-04). On the 12 models that have a Revit export the rule changes nothing; the one project with rooms outside its last phase, Autodesk's `rac_basic_sample_project`, has a public Revit export that holds all 14, which the default leaves out and `--phase "Working Drawings"` writes.

## 1. The question

RE-184 §5 read revit-ifc as skipping a room whose own phase parameter (`ROOM_PHASE`) is set and is not the export phase, which it took to be the last of the project's phases, and applied no rule to rooms because no model showed the parameter to be the room's created phase. Done when that parameter is read and measured on a model whose Revit export leaves rooms out by phase.

## 2. Where a room's phase is stored

`RoomElem` is the native class of rooms, Areas and the records of rooms that are unplaced or not enclosed (RE-312, RE-317, RE-261). Its root object has a field `m_phaseId`, an ElementId, on every release from Revit 2015 to 2027 (`native_document::extract_graphs`, 13 releases of Autodesk's `rac_advanced_sample_project` and `rac_basic_sample_project`). Its Element base `m_createdPhaseId` is -1, unset, on every `RoomElem` record read, so a room's phase is **not** its created phase. No saved parameter of a `RoomElem` is an ElementId naming a phase: the one ElementId parameter read, -1114251, is unset and sits on MEP-space records of the RE1 MEP models.

| `m_phaseId` | records |
|---|---|
| the last phase | every room that Revit's export holds (621, below) and Core Interior's 161 and RE1 Architecture's 2 unplaced records |
| -1, unset | every Area (the office's 30 as built and at permit) and every record of `rac_advanced` that is not one of its 91 rooms (25 on each release), 1 of `rst_basic`'s 31, Snowdon Towers Architectural's 101 and Structural's 7 |
| the second of three phases | Autodesk's `rac_basic_sample_project`: 14 of 14 rooms, on all 13 releases |

Revit exports an Area whatever its phase (RE-184 §5), and an Area's `m_phaseId` is unset, which agrees.

## 3. Rooms in each phase, per model

Phase order and the room phase, by `probe_re326_room_phases` (Measure run 41776212669, ref `0d12ba0e271b04ebd26831a2679c47766bd268aa`, `-f nordic=true -f r2025=true -f samples=true -f bimcollab=true`). "In Revit" pairs each record with Revit's `IfcSpace` of the same GlobalId.

| model (release) | phases | rooms rvt-rs writes | in the last phase | in another phase | in Revit's export |
|---|---|---:|---:|---:|---:|
| Core Interior (2024) | Existing, New Construction | 116 | 116 | 0 | 116 |
| RE1 Architecture (2025) | Existing, New Construction | 11 | 11 | 0 | 11 |
| BIM4LCA office as built (2024) | Olemassa oleva, Uusi rakenne | 234 | 234 | 0 | 234 |
| BIM4LCA office at building permit (2024) | the same | 234 | 234 | 0 | 234 |
| terrain office, concrete and timber (2024), each | the same | 7 | 7 | 0 | 7 |
| R2025 models 12 to 17 (2025), each | Existing, New Construction | 2 | 2 | 0 | 2 |
| RE1 Electrical, Mechanical, Plumbing; Einhoven (2023); the other 17 R2025 models | | 0 | | | |
| Autodesk `rac_advanced_sample_project`, 2015 to 2027 | Existing, New Construction | 91 | 91 | 0 | no export |
| Autodesk `rme_basic` and `rme_advanced` sample projects, 2015 to 2027 | Existing, New Construction | 76 | 76 | 0 | no export |
| Snowdon Towers Architectural (the 2024 sample) | Legends, Existing, New Construction | 54 | 54 | 0 | no export here |
| **Autodesk `rac_basic_sample_project`, 2015 to 2027** | **Existing, Working Drawings, Learning Content** | **14** | **0** | **14, all Working Drawings** | no export in the corpus |

Snowdon Towers Architectural's Revit export is local only and the local copy is the 2025 edition; the 2024 sample's rooms are all in New Construction, its last phase. The held-out models were not read: their first measurement is the regression check in §7. The 12 models with a Revit export hold 621 rooms, all in the last phase and all in Revit's export, so none of them can tell a rule that leaves out a room in another phase from one that does not.

## 4. What revit-ifc does

Read from Autodesk's open-source exporter (source 5 of `CLEANROOM.md`), never copied:

- `ElementFilteringUtil.IsRoomInInvalidPhase` skips an element that is a room when its `ROOM_PHASE` parameter is a valid ElementId different from `ExportOptionsCache.ActivePhaseId`. `Exporter.ExportSpatialElements` calls it for every spatial element and applies nothing else of the phase to it; `ElementFilteringUtil.GetExportFilter` adds the phase status filter for non-spatial elements only. The check is in Autodesk's SourceForge `ifcexporter` v17.4.0 (r126, the version the export in §5 names), in `IFC_v19.2.0` and on `master`.
- `ExportOptionsCache.Create` sets `ActivePhaseId` from the export setup's active-phase option (`IFCExportConfiguration.ActivePhaseId`, unset by default) and from the `VIEW_PHASE` of the filter view, which exists only when the setup limits the export to the elements visible in a view (`IFCExportConfiguration.VisibleElementsOfCurrentView`, false by default). In v17.4.0 and `IFC_v19.2.0` the option comes first and the view's phase fills in when none is named; on `master` the view's phase wins. With neither, the phase is the document's last (`Create` in the older versions, `ExportOptionsCache.UpdateForDocument` on `master`).

So the export phase is the last phase **by default**, and an export setup or a view can name another. RE-184 measured the default on a fresh model (`01_Walls_Phases`), where the default view's phase is the last, and could not tell the three sources apart. rvt-rs reads neither a setup nor a view.

## 5. The export that holds the rooms

A public attachment to a discussion on the IFC for Revit wiki (SourceForge `ifcexporter`, "Exporting Zones", 2018-01-04, posted with the `.rvt` it was made from): `rac_basic_sample_project.ifc`, sha256 `1114ce918f64885bd87ec3bc670c4c543a90898d1bf7e701fb8fbf95e5b5ea35`, 9,129,271 bytes. Its header names exporter 17.4.0.0 for Revit 2018, IFC4 and `ViewDefinition [ReferenceView_V1.0]`, and it holds 17 `IfcSpace`: 14 rooms and 3 MEP Spaces. Its provenance is a forum user's, not Autodesk's, with no licence stated: the file is used locally and is not committed or redistributed, and its settings (the phase, a visible-elements limit) are not in it.

- **The rooms.** The 14 rooms of Autodesk's `rac_basic_sample_project` that rvt-rs reads (101 to 106 and 201 to 208) have the GlobalIds of 14 of its `IfcSpace`, on every release from 2015 to 2023 (the lineage is the same file). All 14 are in Working Drawings, the second of the project's three phases, and the last phase is Learning Content. A rule that leaves out a room not in the last phase leaves out all 14, and Revit's export holds every one.
- **The elements.** `probe_re326_phase_census` (run 41779363950, ref `ecfc16b3677dce5ec6d014c9718c7c1826272dff`, the 2018 sample) reads 550 phased records; every one is created in Working Drawings, none is created or demolished in Learning Content, and 438 are the `Tag` of an element of Revit's export, which has 438 `IfcElement` and no other. Nothing in the project exists only in Learning Content or ends in it, so the elements are the same in an export of Working Drawings and an export of Learning Content, and they do not say which one it was.

So the export either named Working Drawings (a setup or a view, as in §4) or Revit skipped no room in it. The 17.4.0 source has the room check, so the first holds if `ROOM_PHASE` is `m_phaseId`, and that is the reading consistent with every other export here, but nothing in the file decides it.

## 6. What rvt-rs does

- **Reads the phase.** `compute_phase_filter` reads `m_phaseId` of every `RoomElem` record, and each phase's name, in its existing pass (`PhaseFilter::room_phases`, `PhaseFilter::names`). An unset phase, an unread graph and a record whose copies disagree give none.
- **Leaves out a room whose phase is read and is not the export phase.** `PhaseFilter::excludes` takes a room by `room_outside_export_phase`, and every element by the RE-184 status rule against the same `PhaseFilter::export_phase`. A room whose phase is unset, unread or not one of the project's phases is written, and so is an Area or an MEP space, as revit-ifc checks a `Room` only.
- **Names the export phase.** The last phase by default. `RevitFile::set_export_phase` and `rvt-ifc --phase <name|id>` choose another by its name, exactly, or its ElementId; a phase the file does not list is an error that lists the phases it does, and so is a file whose phases are not read (a family, Revit 2015 and earlier, where no phase filter applies). The Python bindings and the viewer take the default.
- **Counts what it leaves out.** The diagnostics' `skipped` has one `room_not_in_export_phase` item for each phase the rooms sit in, with the rooms in `count` and `classes.Room` and the phase's name in `phase`; `--anonymize` replaces the name. The rooms are not in `exported.by_ifc_type.IFCSPACE`.
- **Releases.** Revit 2016 to 2023 apply the rule as they apply the RE-184 element filter (RE-315), and 2024 to 2027 apply it. Revit 2015 and earlier write every room, as every element, because both phase rules start at Revit 2016: the element rule's only evidence on a 32-bit file is a Revit 2018 project, and a room rule that reached further than it would stand on less. Autodesk's `rac_basic_sample_project` saved by Revit 2015 therefore keeps its 14 rooms. One gate covers both rules; #1332 extends them together to Revit 2015 once a 2015 export, or a measured read of a 2015 phase order against one, supports it.

On Autodesk's `rac_basic_sample_project` (phases Existing, Working Drawings, Learning Content; 14 rooms in Working Drawings), measured at that ref, before the fallback of §9, on the 2018 save with `tools/re/spaces_by_globalid_vs_ifc.py` against the public Revit 2018 export of §5 (Measure runs 41784415396 and 41784419457, ref `4532731d535663010438c2b5c00ace97dafc38a3`, `-f samples=true -f sample_years="2018 2018" -f keep_ifc=true`, the second with `-f ifc_flags="--phase 86961"`):

| export | `IfcSpace` rvt-rs writes | in Revit's export (17) | only in Revit's export |
|---|---:|---:|---:|
| default (the last phase, Learning Content) | 0 | 0 | 17 |
| `--phase 86961` (Working Drawings) | 14 | 14 | 3 (the MEP Spaces) |

The default export's diagnostics hold one `room_not_in_export_phase` item (`count` 14, `classes.Room` 14, `phase` "Working Drawings"); the `--phase` export holds none. Every other entity count is the same in the two exports (`IFCSPACE` 0 and 14 is the only difference; 450 and 464 entities in all). So the rule leaves out the 14 rooms Revit's export of this project holds when the export phase is the last, and writes exactly them, none that Revit's export lacks, when it is Working Drawings. What §5 could not say stays unsaid: the forum export holds these rooms either because its export phase was Working Drawings or because its setup did not skip them, and the file does not say which. The unfiltered Measure of this change (run 41784044966, `main` against ref `ecebf060a9c8684edc9c931ae80974036786d760`, `-f base=main -f nordic=true -f r2025=true -f samples=true -f holdout=true`) compared 111 exports: 12 moved, the saves of `rac_basic_sample_project` from Revit 2016 to 2027 (the schema scorer's `sets` 476 to 462 on 239 to 225 objects and types, and 495 to 481 swept solids, the 14 rooms and their bodies in each), and the other 99 are byte for byte as before by `ifc.sha256`, the held-out models (counts only) included. The Revit 2015 save does not move: both phase rules start at Revit 2016 (#1332). A last exports-only check on the final head is recorded in the pull request: its code is that of `4532731d535663010438c2b5c00ace97dafc38a3`, and what followed is documentation.

## 7. Not measured

- **A room outside the last phase under a Revit export made with the default setup.** None exists in the corpus, so the rule's default (the last phase) is revit-ifc's documented behaviour, checked on no export; what is measured is that it changes no room on the 12 models that have one. A Revit export of a project with such a room, with its export setup, would settle whether `ROOM_PHASE` is `m_phaseId` and whether the default is the last phase. An openly licensed model is welcome through the corpus form (#1194).
- **The export setup's phase and a view's.** Neither is read from a file; `--phase` is how the setup's is given.
- **Held-out models.** Not read in this work. Their first measurement is the regression check: their exports under this change, counts only.
- **The 3 MEP Spaces** of the Revit 2018 export, which rvt-rs reads as no room.
- **Python and the viewer.** They export the last phase; a phase option for them is a follow-up.

## 8. Reproduce

```text
gh workflow run measure.yml --ref main -f ref=0d12ba0e271b04ebd26831a2679c47766bd268aa -f base=none \
  -f probe=probe_re326_room_phases -f probe_scorer=room_phases_vs_ifc -f scorers=none \
  -f nordic=true -f r2025=true -f samples=true -f bimcollab=true
gh workflow run measure.yml --ref main -f ref=ecfc16b3677dce5ec6d014c9718c7c1826272dff -f base=none \
  -f probe=probe_re326_phase_census -f scorers=none -f samples=true -f sample_years="2018 2018"
python3 tools/re/phase_census_vs_ifc.py probe.txt rac_basic_sample_project.ifc --classes
gh workflow run measure.yml --ref main -f ref=4532731d535663010438c2b5c00ace97dafc38a3 -f base=none \
  -f samples=true -f sample_years="2018 2018" -f keep_ifc=true -f scorers=none
gh workflow run measure.yml --ref main -f ref=4532731d535663010438c2b5c00ace97dafc38a3 -f base=none \
  -f samples=true -f sample_years="2018 2018" -f keep_ifc=true -f scorers=none -f ifc_flags="--phase 86961"
python3 tools/re/spaces_by_globalid_vs_ifc.py sample-2018-rac_basic_sample_project/model.ifc rac_basic_sample_project.ifc
```

The second run's `probe.txt` of `sample-2018-rac_basic_sample_project`, and the public export above, are the inputs of the last command. The GlobalIds of the 14 rooms are the `global_id` of that probe's room lines in the first run.

## 9. Fallback to the rooms' phase where the last phase holds none (2026-11-05, #1359)

**Status:** a choice by Griffin (2026-11-05, "Fall back to rooms' phase"), not a measurement. No Revit export made at the last phase of a project whose last phase holds no room exists, so what Revit's default writes for such a project is unmeasured. What is measured is the cause, below, and (once the run named in §9.4 finishes) what the change does to the corpus.

### 9.1 The cause

An outside importer reported Autodesk's `rac_basic_sample_project` going from 14 rooms (`decoded_elements()`, `IfcSpace`) in 0.13.0 to 0 in 0.14.0 (#1359). The rule of §6 is the cause: the project's phases are Existing, Working Drawings and Learning Content, all 14 `RoomElem` records have `m_phaseId` Working Drawings, and the last phase holds no room. Measure run 41985023986 (ref `b510ff3c`, the default export of `main`; the `diagnostics.json` of each save of 2016 to 2027 read) holds `IFCSPACE` 0 and `room_not_in_export_phase` 14, all in Working Drawings, on every one; the 2015 save holds `IFCSPACE` 14 and counts none, because both phase rules start at Revit 2016 (#1332). `probe_re326_room_phases` (run 41776212669, ref `458540e0`) read 14 `RoomElem` records on the 2018 and 2023 saves, every one with `m_phaseId` Working Drawings and none unset, and §3 records 14 of 14 on all 13 releases. The importer's other losses (the ANTEPROJETO house and others in #1359) are not read here: whether any of them is this rule is not known.

### 9.2 The rule

Where no phase is named and the project's last phase holds no room, the rooms are judged against the latest phase of the project that holds one (`PhaseFilter::room_fallback`, `room_fallback_phase`), and the rooms of every other phase are left out and counted as before.

- **"Holds no room".** No `RoomElem` record whose `m_phaseId` is read and is one of the project's phases is in the last phase. A room whose phase is unset, unread, or not one of the project's does not count towards holding a room. Two reasons, both from this report: an unset phase is what every Area and every unplaced or not enclosed record carries (§2: the office's 30 Areas, 25 records of `rac_advanced` that are not rooms, 1 of `rst_basic`'s 31, Snowdon Towers' 101), and the filter reads `RoomElem` records without their category, so it cannot tell an unset placed room from an Area; counting them would suppress the fallback on any project that has an Area. Every placed room read in §3 has a set phase, so the two readings coincide on every file measured. They differ only on a project whose last phase holds no readable room but holds a room whose phase did not read: the fallback then brings in earlier rooms the other reading would have left out.
- **Named is exact.** `--phase`, Python's `phase=` and the MCP server's `phase` are exact, including naming the last phase, which writes no room of an earlier phase. `PhaseFilter` tells a named phase from the default (`export_named`), and `RevitFile::set_export_phase` drops the memoised walk when the rooms are judged against another phase than the one named, as when it names the last phase after a default export that fell back.
- **Every other element** follows the last phase as before (RE-184): the fallback moves rooms only.
- **A project with a room in its last phase** has no fallback, so its export and its diagnostics are the bytes they were.
- **The phase the filter reads.** All `RoomElem` records count, placed or not, so an unplaced record whose phase is the last also stops the fallback. That errs towards the output of §6.

### 9.3 What each path does

All of them take one memoised `PhaseFilter` (`RevitFile::phase_filter`), so the rooms they hold are the same:

| Path | After the change |
|---|---|
| `rvt-ifc` | Writes the fallback rooms; prints `rvt-ifc: the project's last phase holds no room, so ...`; the diagnostics hold `room_phase_fallback` (`phase`, `rooms`). `--phase` is exact. |
| `RevitFile.write_ifc`, `rvt_to_ifc`, `export_diagnostics`, `export_diagnostics_json`, `rvt_to_ifc_diagnostics` | The same run; a `RuntimeWarning` names the phase and the rooms and says to pass `phase=`. |
| `RevitFile.schedule_csv` | The document exporter alone; a `RuntimeWarning` (`RevitFile.room_phase_fallback`). |
| `RevitFile.decoded_elements`, `element_counts` | `walker::iter_elements_with_limits`, which reads the same memoised walk, so the rooms are the same. They cannot warn through the export's notes and take no `phase`, which the handle keeps from an export; they warn from `RevitFile.room_phase_fallback` when their result holds a room, and say to export with `phase=`. |
| MCP server | Every tool decodes through `run_export_with_limits` with the model's phase, so the default is the fallback, a phase passed is exact, `get_diagnostics` holds `room_phase_fallback`, and the summary's `notes` hold the same line as `rvt-ifc`. No tool schema changed. |
| Viewer | The wasm export runs the same pipeline; the phase select shows the last phase as the default, labelled with the fallback phase, and cannot name the last phase exactly: its last entry is the default. The downloaded diagnostics replace the phase under `--anonymize`'s rule. |
| `rvt-gltf`, `rvt-elements`, `rvt-info` and other CLIs | They read the same walk; `rvt-gltf` has no `--phase` and prints nothing about the fallback. |

### 9.4 Measured on this head

Not yet. The unfiltered Measure (`-f base=main -f nordic=true -f r2025=true -f holdout=true -f samples=true`) of this change is dispatched and its figures, the models that move and the `rules:` and `mcp answers` lines of `rac_basic_sample_project`, are added here when it finishes. Expected, and to be checked: the 2016 to 2027 saves of `rac_basic_sample_project` each move to 14 `IfcSpace` and 0 `room_not_in_export_phase`, and every other model keeps its `ifc.sha256`.

### 9.5 What the fallback can get wrong

- **A project with no room in its last phase gets earlier rooms that Revit's default might leave out.** Revit's default is unmeasured on such a project. The rooms of a renovation project that sit in an earlier phase and have been demolished or replaced would be written; they cannot overlap each other in the phase they sit in (the reason for the choice), but they could overlap what the last phase builds.
- **Rooms in two earlier phases** write the later phase's and leave out the earlier's, which is counted.
- **The unset-phase reading of §9.2** differs from the other only where a room of the last phase does not read.
- **Not measured:** a Revit export made at the last phase of such a project, or with the default setup; a project other than `rac_basic_sample_project` whose last phase holds no room (none is known in the corpus, §3).

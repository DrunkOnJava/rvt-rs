# RE-184: the phase order is `AllProjectPhases.m_phaseIds`, and Snowdon's 6 slabs exist in no phase after Legends

**Date:** 2026-10-05; rebased onto `main` 5a81e745 and re-measured 2026-10-27; rooms (§5) and the CI note (§6) added 2026-10-30, on review of #1074
**Issues:** #328; backlog items B38 and B46
**Artefacts:** 29 local projects, Revit 2023 to 2027: RE1 Architecture, Core Interior, Autodesk's `rac`, `rst` and `rme` samples, Snowdon Towers (2025 edition, 7 disciplines; 2024 Architectural, re-measured 2026-10-27), Golden Nugget (2), three Japanese samples, `02_2025.zip` model 12 (#356), and `01_Walls_Phases` and `01_Walls_Phases_Demo`, two Revit 2025 models made for this question, each with Revit's IFC4 Reference View export (default settings). Measured locally.
**Probe:** `probes/examples/probe_re184_phase_order.rs`.
**Status:** positive for every element but a room. The phase order, the export phase (the last) and the rule for created and demolished phases are each checked against a Revit export. The exporter now applies the rule (`src/native_phases.rs`, `tests/phase_filter.rs`) to every element but a room (§5). Rooms are not checked, because no model here measures the rule Revit applies to them.

## 1. The question

#328: Revit's export of Snowdon Towers leaves out 6 slabs (2287679 to 2287714) that name the phase Legends. rvt-rs has no phase order to compare phases with: the ids are not in sequence, and the link records' lists disagree with each other.

## 2. Method

The probe reads every native record (`native_document::extract_graphs`, B46):

- **Order.** The project's `AllProjectPhases` record, `m_phaseIds`. Its schema fields are `m_phaseIds` and `m_arrPhasingOverrides`. A family document's own `AllProjectPhases` (`m_famId` not -1, an empty list) is skipped.
- **Names.** Each `ProjectPhase` record's `m_name` and `m_description`.
- **Elements.** Each record's `m_createdPhaseId` and `m_demolishedPhaseId`.

For the last phase `P` in the order, it gives each phased record a status:

- created after `P`;
- demolished in `P` or before it;
- otherwise, existing in `P`.

## 3. Result

| file | phase order | records not in the last phase |
|---|---|---|
| Snowdon Towers Architectural 2025 | Legends 2287613, Existing 32440, New Construction 118390 | **6**: Floors 2287679, 2287686, 2287693, 2287700, 2287707 and 2287714, each created **and demolished** in Legends |
| Snowdon Towers Facades 2025 | Existing, New Construction | 104: walls, sweeps and family instances created in Existing and demolished in New Construction |
| Golden Nugget Architektur 2025 | Phase 1, Phase 2 | 33 walls created in Phase 1 and demolished in Phase 2 |
| `01_Walls_Phases` | Existing 1, New Construction 4, Phase 3 2920 | 0 |
| `01_Walls_Phases_Demo` | the same | 1: wall 2892, created in New Construction, demolished in Phase 3 |
| the other 23 files with phases | 2 or 3 phases | 0 |

Re-run on 2026-10-27 over every distinct local `.rvt` (135 files): the native record path reads 94, 91 of them list phases (Revit 2024 to 2027), and the same four files are the only ones with records not in the last phase. The 41 it does not read (Revit 2019 to 2022, and files with no readable version) get an empty filter, so their exports do not change.

The three 2023 files return neither an order nor any phased record, so the rule does not apply to them.

- **Legends is the first phase, not the last.** That contradicts the reading of #328's 2026-09-23 lead from the link records' lists. Legends' description says why the slabs are never shown: "Phase used for creating legends - elements are created and demolished in this phase". A record created and demolished in one phase exists in no later phase. In New Construction its status is Past (demolished in an earlier phase).
- **The rule finds exactly #328's six ids**, on the same ids in the 2025 edition, and nothing else on Snowdon Architectural (7,303 phased records exist in New Construction).
- On every file with a reference export here (RE1, Core Interior, the `02_2025.zip` models), the rule leaves nothing out, so it cannot remove an element those exports hold.
- **The export phase is the last one.** In `01_Walls_Phases`, wall 2787 is created in Existing, 2789, 2790 and 2892 in New Construction, and 2788 in Phase 3, the last phase. None is demolished. Revit's export, with no phase chosen, holds all five walls. If it had exported New Construction, 2788 would be Future there and left out.
- **An element demolished in the export phase is left out.** `01_Walls_Phases_Demo` is the same model with wall 2892 demolished in Phase 3. Revit's export holds 2787 to 2790 and not 2892, the one wall the rule leaves out.
- Three files list a phase with ElementId 0 (Snowdon Structural, `rst_advanced` 2027, two Japanese samples). Its `ProjectPhase` record is named, so it is a real phase id.

## 4. What this changes

Revit's IFC exporter exports one phase. Elements whose phase status there is New or Existing are exported. Elements that are Past, Demolished, Temporary or Future there are not. With the order known, each status can be computed from an element's two phase ids:

```text
P = the export phase (index in m_phaseIds); c, d = the element's created and demolished phases
exported  ⇔  index(c) ≤ P  and  (d = -1  or  index(d) > P)
```

With `P` the last phase, this leaves out #328's 6 slabs, and only them, on Snowdon Architectural, and wall 2892 on `01_Walls_Phases_Demo`.

It applies to every element but a room, an area and an MEP space (§5). The exporter applies it in this change:

- `native_phases::compute_phase_filter` reads the order and every element's two phase ids in one native pass, memoised as `RevitFile::phase_filter`.
- `native_phases::compute_phase_filter` returns an empty filter without reading anything where the native record path does not read the release (it reads Revit 2023 to 2027). Where it reads the release and the read fails, the filter is empty with `read_error` set, the export writes every element, and the export diagnostics carry a warning ("The project's phases could not be read (...)"), as they do for a stream a resource limit kept from being read whole.
- A new `native_document::Options::graph_classes` decodes graphs only for `AllProjectPhases`. Every other record is read through `native_element::decode_base` alone.
- The filter sits beside `DesignOptions::excludes`, in `without_non_primary_options`, the empty curtain panel scan and the volumeless count. `PhaseFilter::excludes` takes the record, like `DesignOptions::excludes`, and never excludes a room.
- Diagnostics report what it leaves out as `element_record_not_in_export_phase`.

Cost, on a Raspberry Pi 5: Snowdon Architectural's export takes about 2 s longer (on `main` 5a81e745, 43.7 s twice without the filter, 45.2 and 45.7 s with it). The class-only native walk alone is 3 s; decoding every graph would be 20 s.

On the local files the export now leaves out:

| file | left out (skipped item) |
|---|---|
| `01_Walls_Phases_Demo` | 1 wall |
| Snowdon Architectural 2025 | **6**: the Floors 2287679 to 2287714 of #328 (6,044 building elements without the filter, 6,038 with it; the files differ by those six `IfcSlab` and what they own) |
| Snowdon Architectural 2024 (the pinned Autodesk sample, #766) | **6**: the same six Floors, the edition #328 measured (6,056 building elements without the filter, 6,050 with it; only `IfcSlab` changes, 224 to 218) |
| Snowdon Facades 2025 | 104: Wall 42, GenericModel 25, Window 20, WallSweep 11, Roof 6 |
| Golden Nugget Architektur | 30 of its 33 phase-excluded records: Wall 18, Window 6, StairsRun 2, Railing, Roof, Stair and StairsLanding 1 each |
| the other files | 0 |

Snowdon Architectural and Golden Nugget Architektur are read since the ElemTable fix of RE-189 (#760). Without the filter each of #328's six slabs is exported once, and the filter removes exactly those six ids.

## 5. Rooms

Revit's exporter (revit-ifc, `ElementFilteringUtil.cs` and `Exporter.cs`; source 5 of `CLEANROOM.md`) does not apply the status rule above to a spatial element:

- `ElementFilteringUtil.GetExportFilter` adds the phase status filter (None, Existing or New in the export phase) only for non-spatial elements. Its comment says a spatial element gets a check afterwards.
- That check is `ElementFilteringUtil.IsRoomInInvalidPhase`. It applies to an element that is a `Room` and nothing else: it reads the room's `ROOM_PHASE` parameter and skips the room when that is a valid ElementId different from the export phase. `Exporter.ExportSpatialElements` calls it, and so does the second-level space boundary pass in `SpatialElementExporter`.
- The spatial pass collects every `SpatialElement`: rooms, MEP spaces and areas. Only a room is checked, so an area or an MEP space is exported whatever its phase.
- With no phase chosen, the export phase is the last of the document's phases (`ExportOptionsCache.UpdateForDocument`), as §3 found for the other elements.

So a room is exported only when its `ROOM_PHASE` is the last phase, and its created and demolished phases play no part. Whether `ROOM_PHASE` is the room record's `m_createdPhaseId` is a measurement this report does not have: revit-ifc reads the parameter through Revit's API and never names `CreatedPhaseId`, and no measurement here compares a room's phase with a Revit export. `CLEANROOM.md` asks for a finding from revit-ifc to be checked against Revit's export of a real model before it lands, so the room rule is not applied, and a room is never dropped on the guess that the two ids coincide. `PhaseFilter::excludes` never excludes a room, and neither does the skipped item that counts what the export leaves out. What would settle it is a model with a room placed in an earlier phase than the last, with Revit's export of it.

What this leaves: a room created in an earlier phase than the last is written, where Revit's export may leave it out. The status rule excluded no room on any CI model: in Measure run 40930512289 (every model, on the PR's head against `main`), only Snowdon Towers Architectural's export moved, and only its `IfcSlab` count (224 to 218).

Areas are not read through the filter either: `partition_area_records` applies design options only, and MEP spaces are not among the recovered categories.

## 6. Not done

- **The export phase.** It is the last phase in `m_phaseIds` with Revit's default export settings (`01_Walls_Phases`). An export setting that picks another phase, or a filter view whose own phase Revit exports instead, is not read.
- **Rooms** (§5).
- **The test is not run in CI.** `tests/phase_filter.rs` needs `01_Walls_Phases` and `01_Walls_Phases_Demo` with Revit's exports of them. Those models are not published or pinned: their `.rvt` files are in no archive `research/r2025-models.tsv` pins (it notes that `01_Walls_Phases_Demo` has an export but no `.rvt`), so no CI job has them and the test skips there and passes. CI does not run the phase filter against a Revit export until they are published and fetched. Measure moves only Snowdon Towers Architectural's export with the filter (its six slabs, run 40930512289), and Revit's export of that sample is local only, so no Revit export in CI checks it either.

## 7. Reproduce

```text
cargo run --profile ci -p rvt-probes --example probe_re184_phase_order -- "Snowdon Towers Sample Architectural.rvt" RE1-Architecture.rvt
RVT_PROJECT_CORPUS_DIR=<dir holding 01_Walls_Phases*.rvt/.ifc> cargo test --profile ci --test phase_filter
```

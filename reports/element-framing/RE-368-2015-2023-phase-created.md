# RE-368: Revit 2015 to 2023 elements have their Phase Created, from their own `m_createdPhaseId`

**Date:** 2026-11-07
**Issues:** #1190 (geometry and properties on 2015 to 2023, 2026 and 2027); follows #755 and spike 257 (Phase Created on 2024 from the reference list), RE-184 (the phase order and each element's created and demolished phase), RE-281 (that order and those fields read on 2015 to 2022), RE-315 (the export phase applied on 2016 to 2023).
**Artefacts:** Autodesk's own sample projects, pinned in `research/autodesk-sample-projects.tsv` and fetched by `tools/fetch-autodesk-samples.sh`, never stored; BIMcollab's Revit 2015 structural example (RE-198, local use only, not redistributable), for its phases alone.
**Scorer:** `probes/examples/probe_re368_phase_created.rs`, by ElementId, against the same project's 2024 save (2019 for `rst_advanced`).
**Status:** positive against the same samples saved by a later release. **No Phase Created of a 2015 to 2023 file is scored here against a Revit export**: none of the local Revit exports of these releases writes Revit's parameter-group sets, and a later save cannot catch an error it shares. The 2024 rule it is held to was Revit's on 38,555 of 38,555 elements (#755).

## 1. Before

On Revit 2015 to 2023 no element has a `Phasing` set. On `rac_advanced` 2023 that is 5,566 elements the 2024 save gives `Phasing.Phase Created`; on `rme_advanced` 2019, 6,642. 2024's rule (`attach_phases_created`) is the one phase an element's reference list names, by a phase-name frame (`partition_phases::find_phase_names`) measured on 2024 only, and the 2015 to 2023 path does not run it.

## 2. The phase

Every element body carries `m_createdPhaseId` (RE-184), and since RE-281 `native_phases::compute_phase_filter` reads it, with the phases' names (`ProjectPhase.m_name`) and order, on 2015 to 2027. The export already uses it on 2016 to 2023 to leave out what is not in the export phase (RE-315).

An element's Phase Created is now that phase's name, on Revit 2015 to 2023 (`PHASE_CREATED_32BIT_REVIT_VERSIONS`). An element gets none when the field is unset (-1), names no phase of the project's order, or its copies name different phases. A room is left out, as on 2024: its `m_createdPhaseId` is unset (RE-326). 2015 is included though the export phase is not applied there: reading an element's phase is not the filter, and the 2015 samples score as the later ones do (§3).

## 3. Against a later save

`probe_re368_phase_created` prints each element's ElementId and phase, through `recover_partition_schema_mvp`: on 2024 the reference-list rule, on 2015 to 2023 this one. Each save against the same project's 2024 save, on the ElementIds both give a phase:

| | elements both give a phase | equal |
|---|---:|---:|
| `rac_advanced` 2015 to 2020 (each) | 5,499 | 5,499 |
| `rac_advanced` 2021 to 2023 (each) | 5,568 | 5,568 |
| `rac_basic` 2015 to 2023 (each) | 451 | 451 |
| `rst_basic` 2016 to 2018 (each) | 453 | 453 |
| `rst_basic` 2019 to 2023 (each) | 454 | 454 |
| `rme_advanced` 2015, 2019 | 6,630, 6,648 | all |
| `rme_basic` 2015, 2019 | 6,688, 6,693 | all |
| `rst_advanced` 2015, against its 2019 save | 905 | 905 |

**84,045 of 84,045** across the 30 saves held to their 2024 save, none different. `rst_advanced` 2015's 905 are held to its 2019 save, which this same rule reads, so they show only that two releases read alike and are not counted in that figure (corrected 2026-11-08; this section first gave 84,950 of 84,950 across 31 saves). 2015's `rst_basic` is another model (RE-181) and is left out. An element only one side gives a phase is one only that save holds: 18 on `rac_advanced` to 2020 and on `rme_advanced` 2015, 0 to 6 elsewhere; the 2024 `rac_advanced` holds 69 that the saves to 2020 do not.

Each sample puts all its elements in one phase, which is not always the last: `rac_basic`'s are in `Working Drawings`, the second of its three, and the others' in `New Construction`. So the samples show the field is read, and that it is not the export phase, but not that it tells two phases apart.

**Two phases in one file.** BIMcollab's 2015 example orders `Renvooi`, `Renvooi slopen`, `Bestaand`, `Slopen`, `Nieuw`. 241 elements take `Nieuw` and 4 take `Renvooi`. Those 4 are among the 8 elements RE-281 finds outside the last phase, each demolished before `Nieuw` (`compute_phase_filter` gives `Demolished` for all 8). Revit's IFC2X3 export of it holds no `Phasing` set, so the value is not scored, but it is consistent: an element demolished in a later phase was created in an earlier one.

## 4. What else changes

- **Added, none changed.** On the 33 sample saves of 2015 to 2023 the probe gives 86,229 elements a phase, and each of them the export writes gains `Phasing.Phase Created` (5,515 on `rac_advanced` 2015 to 2020, for instance, where 2 of the 5,517 are not written). Elements and their classes, storeys, GlobalIds, hosts, geometry and every other property are as on `main`.
- **Byte identity.** (On 2015 and 2016 exports too the set is added: Measure run 42516796053 shows BIMcollab's 2015 example and `rac_advanced` 2015 to 2020 moving. The sentence that follows is about the 206 local files only.) Of the 206 local project files of 2008 to 2027, 170 export IFC and diagnostics byte for byte as `main` does (`SOURCE_DATE_EPOCH=1700000000`). The 36 that change are files of 2017 to 2023 (none of 2024 or later, none before 2015; the local 2015 files are Autodesk samples, scored in §3, and BIMcollab's). On six of them (`rac_advanced`, `rme_advanced` and `rst_basic` samples, Einhoven, `Table_chair`, a 2019 test model) every product has the same class, Tag, name, storey and property sets but `Phasing`, the only added entities are one `IfcPropertySet`, `IfcPropertySingleValue` and `IfcRelDefinesByProperties` per element given a phase, and the diagnostics differ in `total_entities` alone.
## 5. A lead for #879, not changed here

Of BIMcollab's 4 `Renvooi` elements, 3 are the air terminals #879 counts as written by rvt-rs and left out by Revit's export (`124771`, `148100`, `221553`). All 3 are demolished before `Nieuw`, the export phase, so RE-184's rule leaves them out. RE-281 §2 and RE-315 §10 say none of BIMcollab's 8 such elements is written; that held when they were measured, before these air terminals were recovered. The export phase is not applied on 2015 (`EXPORT_FILTER_RELEASES` is 2016 to 2023), so they are still written. Applying it on 2015 would be a change to which elements are exported, measured on this one file, and is left to its own change.

## 6. Not measured, not read

- Against a Revit export: see the status above.
- `Phase Demolished`: not read here. No sample element is outside its last phase (RE-281), so the samples could not score it.
- `Constraints.Level` and the other Level parameters on these releases, the next-largest property gap against the 2024 saves.
- Revit 2008 to 2014: `compute_phase_filter` does not read them.

## 7. Reproduce

```bash
tools/fetch-autodesk-samples.sh samples 2015 2024
cargo run --profile ci -p rvt-probes --example probe_re368_phase_created -- samples/2023-racbasicsampleproject.rvt > 2023.txt
cargo run --profile ci -p rvt-probes --example probe_re368_phase_created -- samples/2024-racbasicsampleproject.rvt > 2024.txt
diff 2023.txt 2024.txt
```

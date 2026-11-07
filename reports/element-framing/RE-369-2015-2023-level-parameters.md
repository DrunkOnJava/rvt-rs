# RE-369: Revit 2015 to 2023 elements have their Level parameters, from their own native record

**Date:** 2026-11-07
**Issues:** #1190 (geometry and properties on 2015 to 2023, 2026 and 2027); follows RE-172 (Level parameters on 2024 from the per-element association record), RE-174 and RE-175 (a stair's Levels and a beam's Reference Level from the reference lists), RE-281 (the Element base and saved parameters read on 2015 to 2022), RE-368 (Phase Created on 2015 to 2023).
**Artefacts:** Autodesk's own sample projects, pinned in `research/autodesk-sample-projects.tsv` and fetched by `tools/fetch-autodesk-samples.sh`, never stored.
**Scorer:** `probes/examples/probe_re369_levels.rs`, by ElementId, against the same project's 2024 save (2019 for `rst_advanced`).
**Status:** positive against the same samples saved by a later release. **No Level parameter of a 2015 to 2023 file is scored here against a Revit export**: none of the local Revit exports of these releases writes Revit's parameter-group sets, and a later save cannot catch an error it shares. The 2024 rule it is held to equalled Revit's own IFC4 export on every element tested in seven files (RE-172).

## 1. Before

On Revit 2015 to 2023 no element has a `Constraints` set. Against each sample's 2024 save it is the largest property gap after Phase Created (RE-368): 3,276 mullions, 1,349 panels, 176 columns, 146 walls and about 400 other elements on `rac_advanced` 2023, 370 beams on `rst_basic` 2023. 2024's rule reads the association record `d3 02 01 00 00 00` (RE-172), which is measured on 2024 only, and the 2015 to 2023 branch of `recover_partition_schema_mvp` does not run it.

## 2. The Levels a record names

Every native record opens with the Element base, whose `m_assocLevelId` RE-281 reads on 2015 to 2027. RE-281 §4 found it names the storey Revit's export puts the element in on BIMcollab's 2015 example (210 of 210). It is the element's Level, Base Constraint or Base Level, as RE-172's `LevelId` is on 2024. Where it is unset, the record's own graph names the Level in a field of its class:

| class | field | the export writes it as |
|---|---|---|
| any element | Element base `m_assocLevelId` | `Level`, a wall's `Base Constraint`, a column's `Base Level` and `Level`, a railing's, roof's or stair's `Base Level` (RE-172's names by class) |
| `SWall`, `ArcWall` | `m_upToLevelId` | `Top Constraint`, `Up to level: <name>` |
| `Roof` | `m_baseLevelId` | `Base Level` |
| `StairsElement` (assembled stair) | `m_baseLevelId`, `m_topLevelId` | `Base Level`, `Top Level` |
| `Stairs` (sketched stair) | `m_bottomId`, `m_topId` | `Base Level`, `Top Level` |
| `FamilyInstance` | `m_scheduleOnlyLevelId` | `Level` (light fixtures, wall-hosted devices) |
| `FamilyInstance` | `FamInstCurveDriver.m_refLevelId` | a beam's or brace's `Reference Level` |
| `RbsDuctCurve`, `RbsPipeCurve` | `m_idLevelStart` | `Reference Level` |

`attach_levels_32` reads them on Revit 2015 to 2023 (`LEVEL_32BIT_REVIT_VERSIONS`) in two passes over the elements the branch recovers: every record's Element base, with no graph built (as `compute_phase_filter` reads it), then the graphs of walls and of the elements whose base names no Level, each projected to the fields above for its class (`graph_fields`, #1335). A Level is written only where it is one the branch recovers (`partition_level_records`) and every copy of the record names the same one; anything else gives nothing. The `Constraints` set is written by the same code as on 2024 (`level_parameter_property_set`), which now also takes a beam's or brace's Reference Level from `REFERENCE_LEVEL_FIELD`.

## 3. Against a later save

`probe_re369_levels` prints each element's Level parameters through `recover_partition_schema_mvp`: on 2024 RE-172's, RE-174's and RE-175's rules, on 2015 to 2023 this one. `level` is the one the export writes as `Level`, `Base Constraint`, `Base Level` or `Reference Level`, by class; `top` a wall's `Top Constraint` or a stair's `Top Level`. Each save against the same project's 2024 save, on the (ElementId, kind) pairs both give:

| | parameters both give | equal | of which `top` |
|---|---:|---:|---:|
| `rac_advanced` 2015 to 2020 (each) | 5,640 | 5,640 | 177 |
| `rac_advanced` 2021 to 2023 (each) | 5,650 | 5,650 | 182 |
| `rac_basic` 2015 to 2023 (each) | 485 | 485 | 51 |
| `rst_basic` 2016 to 2018 (each) | 441 | 441 | 10 |
| `rst_basic` 2019 to 2023 (each) | 442 | 442 | 10 |
| `rme_advanced` 2015, 2019 | 6,574, 6,592 | all | 173 |
| `rme_basic` 2015, 2019 | 6,631, 6,636 | all | 173 |

**85,121 of 85,121 across 30 saves, none different.** `rst_advanced` has no 2024 save; its 2015 save against its 2019 save, both read by this rule, gives 583 of 583, which shows only that the two releases are read alike. 2015's `rst_basic` is another model (RE-181) and is left out.

The same on what the export writes, by (IFC class, Tag), against the export of the 2024 save: 86,458 property values equal, none different, none only in the older save's export. 336 are only in the 2024 save's: the `Reference Level` of 23 beams on each `rac_advanced` and 15 on each `rst_basic`, whose base and `FamInstCurveDriver.m_refLevelId` are both unset (2024 takes it from the highest Level the reference list names, RE-175), and the `Level` of one bathtub on each `rac_basic`, which names a Level in none of the fields read. They are left without a value.

In the probe's counts, a parameter only the 2024 save gives is one of those beams or that bathtub, or, on the MEP samples, the Level of 19 or 20 cable trays and conduits, which the export writes for neither save (RE-319), of 18 duct parts the 2015 `rme_advanced` export does not write, or of up to 4 elements neither export writes. A parameter only the older save gives (up to 18 on a save) belongs to an element only that save holds or reads.

## 4. What else changes

- **Added, none changed.** On the 33 sample saves of 2015 to 2023 every product the export writes keeps its class, Tag, GlobalId, name, storey and every property set but `Constraints`; the only added entities are one `IfcPropertySet`, `IfcRelDefinesByProperties` and one or two `IfcPropertySingleValue` per element given a Level (5,473 elements on `rac_advanced` 2015 to 2020, 6,449 on `rme_basic` 2019); the diagnostics differ in `total_entities` alone. The 2024 saves export byte for byte as on `main`.
- **Byte identity.** Of the 206 local project files of 2008 to 2027, 172 export IFC and diagnostics byte for byte as `main` does (`SOURCE_DATE_EPOCH=1700000000`). The 34 that change are files of 2017 to 2023; none of 2024 or later, none before 2015. Einhoven (2023) does not change: the branch recovers no Level record there, so nothing is written.
- **Time.** The two passes take 70 to 330 ms on the samples (the base pass 8 to 57 ms; most of it is the projected graphs of the MEP samples' family instances, ducts and pipes).

## 5. Not measured, not read

- Against a Revit export: see the status above.
- Beams and braces whose `m_refLevelId` is unset: 2024 takes their Reference Level from the highest Level their reference list names (RE-175), not measured on the 32-bit lists.
- Flexible ducts and pipes, cable trays and conduits: no sample holds one the export writes with a Level; a conduit's is not written on any release (RE-319).
- Revit 2008 to 2014: the native record path does not read them.

## 6. Reproduce

```bash
tools/fetch-autodesk-samples.sh samples 2015 2024
cargo run --profile ci -p rvt-probes --example probe_re369_levels -- samples/2023-racbasicsampleproject.rvt > 2023.txt
cargo run --profile ci -p rvt-probes --example probe_re369_levels -- samples/2024-racbasicsampleproject.rvt > 2024.txt
diff 2023.txt 2024.txt
```

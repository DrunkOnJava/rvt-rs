# RE-287: a stair's flight set and its TreadLengthAtInnerSide

**Date:** 2026-10-30
**Issues:** [#1186](https://github.com/DrunkOnJava/rvt-rs/issues/1186), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-47, RE-92 and RE-236
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, CC BY-SA 4.0) with Revit's IFC4 exports (exporter 24.0.5.432), and the five stair models of puzzbobb's R2025 set (models 13 to 17, Revit 2025, Apache-2.0) with their IFC4 exports (exporter 25.0.2.419), as Measure fetches them
**Run:** Measure [41137371839](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41137371839) (`-f base=main -f nordic=true -f r2025=true -f ifc_flags=--revit-set-placement`, every model and scorer, commit a0ec6e5d against `main` f73908bf)
**Status:** positive, measured. The rule that gives a stair's flight set is exact on every stair measured (14 on each office file, 5 on the R2025 models); the mechanism in revit-ifc that produces it is not read.

## 1. What was missing

Revit's export writes two sets on each `IfcStair` of the office: `Pset_StairCommon` and `Pset_StairFlightCommon` (the second by revit-ifc's matching of a set to an entity by the start of the entity's name, RE-235). rvt-rs wrote the first except `TreadLengthAtInnerSide` and the second only on the flights. So 14 `Pset_StairCommon.TreadLengthAtInnerSide` and 98 `Pset_StairFlightCommon` values (seven properties on each of 14 stairs) were missing on each office file, and one of each on every R2025 stair model.

## 2. What Revit writes

On the 14 office stairs that hold values (the other two, the family stairs 3317210 and 3317486, hold only `Reference` and `IsExternal`), each stair's `Pset_StairFlightCommon` against the flights aggregated under it (Revit's export, as built; permit is the same):

| property on the stair | equals, on 14 of 14 stairs |
|---|---|
| `NumberOfRiser` | the sum of its flights' (3; 7; 10 + 9 = 19; 8 + 11 + 8 = 27 on ten stairs; 19) |
| `NumberOfTreads` | the sum of its flights' (2; 6; 9 + 8 = 17; 7 + 10 + 7 = 24 on ten stairs; 18) |
| `RiserHeight`, `TreadLength`, `NosingLength`, `TreadLengthAtOffset`, `TreadLengthAtInnerSide` | the flights' value, the same on every flight of a stair (riser heights agree to 1e-9 relative) |

It is not the stair's own `Pset_StairCommon`: stair 1503002 has 5 risers, 4 treads and a riser height of 128.6 mm there, and 7, 6 and 114.29 mm in its flight set (its one flight's); stair 2598347 has 19 risers and 18 treads there and 19 and 17 in its flight set. Each R2025 stair model's one stair follows the same rule.

## 3. Revit's calculators, and what they do not explain

Read in revit-ifc at `IFC_v24.2.0` (CLEANROOM.md source 5; read and cited, no code copied): `NumberOfRiserCalculator` and `NumberOfTreadsCalculator` give a `Stairs` element its `ActualRisersNumber` and `ActualTreadsNumber` and a `StairsRun` its own; `RiserHeightCalculator` gives a `Stairs` its `ActualRiserHeight` and a run its height over its risers; `TreadLengthCalculator`, `TreadLengthAtOffsetCalculator` and `NosingLengthCalculator` give both the stair's tread depth and its first run type's nosing. So run on the stair, the calculators give the `Pset_StairCommon` values, and that is what the stair holds there. They do not give the container's flight set: sums of the runs' counts and the runs' riser height are what the runs give, and where in the exporter the container's flight set is filled from them is not read. The rule of §2 is measured only, on 14 stairs of one to three flights on each office file and on five stairs of the R2025 models, none of which differs; a stair whose flights disagree on a length is outside it, and rvt-rs writes nothing for that property.

`TreadLengthAtInnerSideCalculator` takes, for a `Stairs` and for a `StairsRun` alike, the stair type's `STAIRSTYPE_MINIMUM_TREAD_WIDTH_INSIDE_BOUNDARY` where it is above zero, else the stair's `ActualTreadDepth`. rvt-rs reads no stair type's inside-boundary width. Its flights' `TreadLengthAtInnerSide` has been the tread depth (RE-47), and it is Revit's on 49 of 49 flights on each office file and on the R2025 models, so no stair type measured sets one. A stair's is written the same way; where a stair type sets one, Revit's value would differ from rvt-rs's, for the flights and the stair alike.

## 4. What rvt-rs does

- `stair_property_set` writes `TreadLengthAtInnerSide` in a stair's `Pset_StairCommon`, as it does for a flight's set: the stair's tread depth.
- `revit_set_placement::stair_flight_additions`, with `rvt-ifc --revit-set-placement` (IFC4 applies `Pset_StairFlightCommon` to flights, not stairs): for each `IfcStair` with flights aggregated under it, `NumberOfRiser` and `NumberOfTreads` as the sums over its flights where every flight holds the count, and each length as the flights' where every flight holds it and they agree to 1e-9 relative. Nothing for a property a flight lacks.

## 5. Measured

`psets_vs_ifc`, head against `main` (run 41137371839). Only the two office files and the five stair models change; every other model, Core Interior and RE1 included, is byte-identical.

| | office as built | office at permit | R2025 models 13, 14, 15, 16, 17 |
|---|---:|---:|---|
| `Pset_StairCommon.TreadLengthAtInnerSide` equal | 0 to 14 of 14 | 0 to 14 of 14 | 0 to 1 of 1 each |
| `Pset_StairFlightCommon.RiserHeight`, `TreadLength`, `TreadLengthAtOffset`, `TreadLengthAtInnerSide` equal | 35 to 49 of 49 each | 35 to 49 of 49 each | 1 to 2 of 2 each |
| `Pset_StairFlightCommon.NumberOfRiser` equal | 35 to 49 of 52 | 35 to 49 of 52 | 1 to 2 of 3 each |
| `Pset_StairFlightCommon.NumberOfTreads` equal | 34 to 47 of 52 | 34 to 47 of 52 | 1 to 2 of 3 each |
| `Pset_StairFlightCommon.NosingLength` equal | 35 to 49 of 54 | 35 to 49 of 54 | 1 to 2 of 4 on models 13 to 16, unchanged on 17 |
| all values equal / different / missing, `main` | 45,088 / 1 / 436 | 45,270 / 1 / 433 | 107 / 0 / 54, 107 / 0 / 54, 107 / 0 / 54, 105 / 0 / 48, 103 / 0 / 50 |
| all values equal / different / missing, this change | 45,199 / 1 / 325 | 45,381 / 1 / 322 | 115 / 0 / 46, 115 / 0 / 46, 115 / 0 / 46, 113 / 0 / 40, 110 / 0 / 43 |
| `diagnostics.decoded.parameter_value_count` | 45,375 to 45,486 (+111) | 45,561 to 45,672 (+111) | +8, +8, +8, +8, +7 |

The one office value that differs is the mullion whose `Span` differs, unchanged. The values written are exactly the rows' gains: 111 on each office file (14 stairs of eight properties, less the spiral stair's `NumberOfTreads`), 8 on each R2025 stair model but the spiral one, which has 7.

## 6. What is left out

- **The assembled (spiral) stair's `NumberOfTreads`**: 18, in the stair's `Pset_StairCommon` and `Pset_StairFlightCommon` and in its one flight's (stair 3057027 and flight 3057703, 3 values on each office file; one stair and flight on the spiral R2025 model). rvt-rs reads no tread count for an assembled stair's run, so the stair's flight sum has no term, and nothing is written.
- **The stair types' and flight types' dimensions** (`Pset_StairCommon` and `Pset_StairFlightCommon` on `IfcStairType`, `NosingLength` on `IfcStairFlightType`): type-object values, RE-286.
- **`WalkingLineOffset`**: half the run's width in feet, which Revit's export scales as millimetres (RE-235 §5), counted apart and not written.
- **The family stairs 3317210 and 3317486**, which hold no dimensions in Revit's export either.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=a0ec6e5d9c6273c35a25976d820839ea18aaa70b -f base=main \
  -f nordic=true -f r2025=true -f ifc_flags=--revit-set-placement \
  -f psets_missing="Pset_StairCommon.TreadLengthAtInnerSide,Pset_StairCommon.NumberOfTreads,Pset_StairFlightCommon.NumberOfTreads,Pset_StairFlightCommon.NumberOfRiser"
```

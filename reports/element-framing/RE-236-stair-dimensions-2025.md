# RE-236 — A Revit 2025 stair's and flight's dimensions

**Date:** 2026-10-25
**Issues:** #35 (parameters), #357 (stairs), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md)
**Artefacts:** puzzbobb's five R2025 stair models (Revit 2025, Revit's IFC4 exports), as Measure fetches them
**Runs:** Measure [40245194800](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40245194800) (`examples/probe_re231_record_lengths.rs` on stairs and flights, with `tools/re/stored_lengths_vs_psets.py`) and [40246435413](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40246435413) (this change against `main`)
**Result:** positive: every stair and flight dimension Revit's export writes on these models and rvt-rs had none of, but where Revit's value is a known defect.

## 1. What was missing

RE-47 reads a stair's riser height, tread depth and risers from its serialised data on Revit 2024 only. On the R2025 stair models rvt-rs wrote none of `Pset_StairCommon`'s or `Pset_StairFlightCommon`'s dimensions.

## 2. The native records

The probe prints every number of each stair's and flight's native record and its type's, and the scorer ranks them against every length and count property Revit writes:

| property | stair | flight |
|---|---|---|
| `NumberOfRiser` | `StairsElement.m_actualNumberOfRisers` | `StairsRun.m_topRiserIndex` less `m_baseRiserIndex` |
| `RiserHeight` | `StairsElement.m_actualRiserHeight` | the run's height (`m_topElevation` less `m_bottomElevation`) over its risers |
| `TreadLength`, `TreadLengthAtOffset`, `TreadLengthAtInnerSide` | `StairsElement.m_actualTreadDepth` | its stair's |
| `NumberOfTreads` | | its risers, one fewer, plus one for each end that is a tread (RE-92's rule, from `m_beginWithRiser` and `m_endWithRiser`) |
| `NosingLength` | the one its runs share | its run type's `m_nosingLength` |

These are revit-ifc's rules for a stair and a run (CLEANROOM.md source 5): its riser height for a run is the run's height over its actual risers, and a run's tread length its stair's actual tread depth. On the model whose run does not end with a riser, 8 risers give 8 treads; on the others 9 give 8. `WalkingLineOffset` is left out, as on Revit 2024 (#1000): Revit's export writes half the run's width in feet read as millimetres.

## 3. Measured

`psets_vs_ifc`, this change against `main`:

| model | equal before | after | different | Revit values rvt-rs does not write |
|---|---:|---:|---:|---:|
| 13 Rooms_Stairs | 84 | 96 | 1 (unchanged) | 0 |
| 14 endwithriserOFF | 84 | 96 | 1 | 0 |
| 15 NoRisers | 84 | 96 | 1 | 0 |
| 16 Monolithic | 82 | 94 | 1 | 0 |
| 17 Spiral | 82 | 92 | 1 | 0 |

The spiral model's run has no native run record rvt-rs reads (RE-217 §11), so its flight and its stair have no nosing length. The flight values still missing on each model are the `Pset_StairFlightCommon` Revit writes on the `IfcStair` itself (RE-235). The BIM4LCA office, Core Interior and RE1 export byte for byte as before.

## 4. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=main -f base=none -f r2025=true \
  -f probe=probe_re231_record_lengths -f probe_scorer=stored_lengths_vs_psets
```

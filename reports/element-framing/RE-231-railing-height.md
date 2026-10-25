# RE-231 — A railing's height, and the length properties no stored value gives

**Date:** 2026-10-25
**Issues:** #35 (parameters), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit's IFC4 exports), RE1 Architecture and Plumbing (Revit 2025, Revit's IFC2X3 exports) and puzzbobb's five R2025 stair models (Revit 2025, Revit's IFC4 exports), as Measure fetches them
**Runs:** Measure [40241368996](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40241368996) (`examples/probe_re231_stored_lengths.rs`), [40243019158](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40243019158) (`examples/probe_re231_record_lengths.rs`), both with `tools/re/stored_lengths_vs_psets.py`, and [40244714948](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40244714948) (this change against `main`)
**Result:** positive for railings: `Pset_RailingCommon.Height` is the railing type's top rail height, Revit's on every railing measured. Negative for the office's beams' `Span` and pipes' `InvertElevation`.

## 1. What was missing

Revit's export writes `Pset_RailingCommon.Height` on every railing of the BIM4LCA office, 4 ft on 28 and 0.9 m on 13, on RE1 Architecture's one and on both railings of each R2025 stair model, and rvt-rs wrote none. The office's beams' `Pset_BeamCommon.Span` (8) and pipes' `Pset_PipeSegmentOccurrence.InvertElevation` (12) were missing too.

## 2. Stored lengths

`probe_re231_stored_lengths` prints every length entry (`f64 · ff×8 · i64`, RE-93) stored under each element and under its type, and the scorer counts, per property, the entries equal to Revit's value within 0.001 ft. A railing type's entry under a key whose low word is 6 holds 4 ft for the 28 railings Revit gives 4 ft. The other 13, ten of them of type 1468014, have no such entry: the type stores no length entry at all.

## 3. Native records

`probe_re231_record_lengths` prints every number at the top level of the first 200 objects of each element's native record and its type's. One member equals Revit's `Height` on every railing:

| file | railings | type's `StairsRailingAttr.m_topRailHeight` = Revit's `Height` |
|---|---:|---:|
| office as built | 41 | 41 |
| RE1 Architecture | 1 | 1 (and `NonContinuousRailInfo.m_height`) |

No member of a beam's or pipe's record, or its type's, equals Revit's `Span` (18.6516 ft on all 8 beams, which rvt-rs draws as boxes) or `InvertElevation` (181.4987 ft on all 12 pipes). On RE1 Plumbing, IFC2X3's `InvertElevation` is a pipe's `RbsPipeCurve.m_dOffsetStart` on 14 of 63. revit-ifc's `InvertElevation` has no calculator in its source (CLEANROOM.md source 5), so it is the built-in "Invert Elevation" parameter, which the record does not hold.

## 4. What rvt-rs does

`native_stairs::railing_type_heights` reads the top rail height of each railing's type (the type RE-66 and B45 join it to) from the type's native record, on Revit 2024 and 2025, and `add_railing_height` writes it as the railing's `Pset_RailingCommon.Height`.

## 5. Measured

`psets_vs_ifc`, this change against `main`:

| file | `Height` equal | different | missing |
|---|---:|---:|---:|
| office as built | 41 of 41 | 0 | 0 |
| office at permit | 41 of 41 | 0 | 0 |
| RE1 Architecture | 1 of 1 | 0 | 0 |
| each R2025 stair model (13 to 17) | 2 of 2 | 0 | 0 |

Nothing else moves, and no value Revit's export does not write is added. Core Interior, Einhoven and RE1 Electrical, Mechanical and Plumbing export byte for byte as before.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=main -f base=none -f nordic=true -f r2025=true \
  -f probe=probe_re231_record_lengths -f probe_scorer=stored_lengths_vs_psets
```

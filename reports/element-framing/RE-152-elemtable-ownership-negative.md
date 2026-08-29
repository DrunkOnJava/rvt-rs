# RE-152 — ElemTable trailing-owner hypothesis (magnetar negative)

**Date:** 2026-08-29  
**Issue:** [#152](https://github.com/DrunkOnJava/rvt-rs/issues/152)  
**Credit:** Reported by [@STE1200](https://github.com/STE1200) in
[Discussion #112](https://github.com/DrunkOnJava/rvt-rs/discussions/112).

## Claim under test

`Global/ElemTable` body is an ownership tree; trailing `u32` (28-byte /
2014-era layouts) or trailing `u64` (40-byte / 2026-era layouts) encodes
the owner ElementId.

## Method

Parse `Global/ElemTable` with the existing layout detector and inspect:

- trailing `u32` / trailing `u64` high half
- every aligned payload `u32` after the marker for coincidence with other
  declared ids

Probe: `examples/probe_elem_table_ownership.rs`.

## Independent result on magnetar corpora

| File | Stride | Parsed records | Trailing `u32` == 0 | Trailing `u32` ∈ id set |
|------|--------|----------------:|--------------------:|------------------------:|
| `Revit_IFC5_Einhoven.rvt` (2023) | 28 B | 2614 | **100%** | **0%** |
| `2024_Core_Interior.rvt` (2024) | 40 B | 26424 | **100%** | **0%** |

Early records are marker + id_primary + id_secondary + **zero payload**,
matching `docs/elem-table-record-layout-2026-04-21.md`.

On 2024, some *interior* payload words coincide with other declared ids
at high rates (~88–92%). That is **not** treated as ownership confirmation
(dense 1…N id space; no independent parent oracle).

## Status

- Trailing-word-as-owner is **falsified** on these two magnetar projects.
- Stride detection (28 vs 40) remains independently confirmed.
- Issue [#152](https://github.com/DrunkOnJava/rvt-rs/issues/152) stays
  **open**: Steffen's 2014-family / 2026-project samples may still carry
  non-zero ownership fields we do not have in-corpus.

**No decoder changes** from this pass.

# RE-175: a large ElemTable with a few ids out of order was read as 12-byte records

**Date:** 2026-10-05
**Issues:** #85 (found while sweeping its tuples); #152 (the layout detector)
**Artefacts:** 174 local models, Revit 2008 to 2027 (Autodesk's samples, Snowdon Towers, Golden Nugget, RE1, and models made for #356 and #328). Measured locally.
**Fix:** `elem_table::detect_layout`, `MAX_OUT_OF_ORDER_PERCENT`.
**Status:** positive. Six large Revit 2025 models gain their 40-byte layout, and the other 162 paths read the same layout as before.

## 1. The question

`detect_layout` takes the record size (28 or 40 bytes) at which the table's stated count of records fits from `0x06` with every id rising, and falls back to the 12-byte implicit layout when neither fits. Does it find the layout of every model?

## 2. Method

For each model the layout is read with `elem_table::read_layout`, on the unfixed detector and on the fixed one. For the six models that fell back, the ids of the stated records are counted that do not rise, at 28 and at 40 bytes.

## 3. Result

12 of the 174 paths fell back to the implicit layout. They are six models, each present twice (`newtest/2019/` holds 2025-written copies):

| model | records | stride 40: tail, ids out of order | stride 28: ids out of order |
|---|---:|---|---:|
| Snowdon Towers Architectural 2025 | 50,529 | 24 B, 3 | 32,694 |
| Snowdon Towers Structural 2025 | 16,612 | 24 B, 2 | 10,557 |
| `サンプル意匠` (Autodesk Japan sample) | 44,157 | 24 B, 163 | 27,725 |
| `サンプル設備` | 50,122 | 24 B, 22 | 31,917 |
| Golden Nugget Gebäudetechnik | 41,559 | 24 B, 34 | 26,537 |
| Golden Nugget Architektur | 49,449 | 24 B, 14 | 32,381 |
| *Snowdon Towers Electrical 2025 (detected before)* | 19,810 | 24 B, 0 | 12,694 |

- **They are 40-byte tables.** Each fits 40-byte records exactly: the stated count from `0x06`, then the same 24 trailing bytes the models that were detected have. At 28 bytes 63% to 66% of the ids are out of order.
- **A few ids sit out of order**, 0.004% to 0.37% of the records. The detector required none, so these models took the fallback. The "records" it then returned are 12-byte slices of 40-byte records.
- **The fallback hid the failure.** It returns the stated number of records, so `parse_records` and `declared_ids` succeed, with ids that are not ElementIds. Every ElemTable-derived fact on these models is wrong.

On `rvt-ifc` the effect is an empty export. Both columns are built from upstream `f822dc6`, the second with only this change:

| model | building elements before | after | IFC before | after |
|---|---:|---:|---:|---:|
| Snowdon Architectural | 1 | 6,035 | 3.2 KB | 16.9 MB |
| Snowdon Structural | 0 | 1,282 | 1.8 KB | 2.4 MB |
| `サンプル意匠` | 0 | 5,827 | 2.0 KB | 16.0 MB |
| `サンプル設備` | 1 | 4,793 | 3.6 KB | 13.9 MB |
| Golden Nugget Gebäudetechnik | 0 | 5,422 | 1.9 KB | 15.6 MB |
| Golden Nugget Architektur | 0 | 2,577 | 1.9 KB | 6.6 MB |

Facts that were reported about these models while the layout was wrong, because their exports were empty, need re-checking: for example, that rvt-rs writes no slabs on Snowdon Architectural 2025 (it writes 6,035 elements, the six Legends slabs of #328 among them), and that the Golden Nugget models export no building elements.

## 4. What this changes

`detect_layout` accepts a record size whose ids rise on all but at most 1% of the steps (`MAX_OUT_OF_ORDER_PERCENT`). Below 100 records that is none, as before. The most out of order at the right size is 0.37%, and the least at the wrong one 63%, so the share separates them.

Run over all 174 paths, the fixed detector changes exactly those 12 and no other: 49 paths stay 28-byte and 113 stay 40-byte, and none falls back.

Tests: `tests/elem_table_corpus.rs`, `fuzz_regressions`, `graceful_degradation` and `native_current_records` give the results they gave on `main`. Of `elem_table_corpus`, `declared_element_ids_returns_sorted_deduped_set` fails the same way before the change: it asserts Einhoven's first declared id is 1 and gets 0. The six models are not in CI's corpora, so no CI test covers them; this report holds the evidence, as RE-40's does for Snowdon.

## 5. Not done

- **Why the ids are out of order.** They are 2 to 163 records of tables of 16,612 to 50,529. Whether they are elements added after the table was first sorted is not looked at.
- **Ten old files** (2008 to 2013) do not open at all: `Malformed BasicFileInfo: no 4-digit Revit version found`. That is a separate reader question.

## 6. Reproduce

```text
rvt-elem-table "Snowdon Towers Sample Architectural.rvt" | grep layout
```

It prints `layout: Explicit (40 B stride, 8-byte owner field ...)` with the change, and an implicit 12-byte layout without it.

# RE-155: an element record's reference list has no fixed slot for its type or Level

**Date:** 2026-10-03
**Issues:** #228
**Artefacts:** Core Interior (Revit 2024) and the four RE1 models (Revit 2025). Measured on GitHub's hosted runners with the Measure workflow, runs 37138525161 and 37138809159. Einhoven was skipped: it has no bbox marker, so its records do not decode.
**Probe:** `examples/probe_re155_reference_slots.rs`.
**Status:** answered. Where a list holds the element's type or a Level, the slot is fixed within a category, but it differs between categories and between releases; the leading 3 is common, not constant.

## 1. The question

RE-23 found that every Revit 2024 element record carries a counted `u64` reference list at +0x88, and that the slot before the record's own ElementId is the host wall. #228 asks what the other slots hold: a leading 3, and ids that look like family, type and Level references.

The Level part is already answered. RE-27 binds an element to the one Level its record names, and RE-59 to the base constraint of a record naming two; on Core Interior all 854 elements matched by `Tag` are in Revit's storey. This report places the slots.

## 2. Method

For every record of every partition's leading chain (RE-35) whose frame decodes, each slot is classified by what it equals, in this order:

- `three`: the value 3 in the first slot;
- `own`: the record's own ElementId;
- `type`: the element's type, as the partition name entries give it (RE-38);
- `level`: a Level's ElementId (RE-24);
- `declared`: an id the ElemTable declares;
- `other`.

Each slot is placed by where it sits: `slot k` counted from the start, `before own`, `own`, or `after own +k`. A slot is counted as `type` only when the element's type is known, which it is for 854 of Core Interior's 22,339 records. The `type` counts are therefore lower bounds; a type id in a record whose type is not known counts as `declared`.

## 3. Result

| model | records | slot 0 is 3 | slot 0 is a Level |
|---|---:|---:|---:|
| Core Interior (2024) | 22,339 | 17,543 | 18 |
| RE1 Architecture (2025) | 152 | 0 | 90 |
| RE1 Mechanical (2025) | 50 | 42 | 0 |
| RE1 Plumbing (2025) | 136 | 126 | 0 |
| RE1 Electrical (2025) | 13 | 2 | 0 |

Where the type and the Level sit, per category, among the records that carry them:

| model | category | records | type | Level |
|---|---|---:|---|---|
| Core Interior | walls (-2000011) | 1,210 | slot 1 (183) | slot 2 (348), slot 3 (195) |
| Core Interior | doors (-2000023) | 200 | slot 1 (120), slot 2 (12) | slot 3 (120) |
| Core Interior | columns (-2000100) | 392 | slot 3 (256) | slot 4 (245), before own (267) |
| Core Interior | rooms (-2000160) | 286 | none | slot 1 (116) |
| RE1 Architecture | furniture (-2000080) | 13 | before own (13) | slot 0 (13) |
| RE1 Architecture | casework (-2001000) | 10 | before own (8), slot 4 (2) | slot 0 (10) |
| RE1 Architecture | plumbing fixtures (-2001160) | 7 | before own (7) | slot 0 (7) |
| RE1 Architecture | specialty equipment (-2001350) | 9 | before own (8), slot 4 (1) | slot 0 (9) |
| RE1 Architecture | mullions (-2000171) | 19 | after own +1 (10) | slot 0 (10) |
| RE1 Mechanical | ducts (-2008000) | 13 | slot 3 (13) | slot 1 (13) |
| RE1 Mechanical | pipes (-2008044) | 3 | slot 4 (3) | slot 1 (3) |
| RE1 Mechanical | duct fittings (-2008010) | 14 | slot 7 or later (13), slot 5 (1) | slot 1 (14) |
| RE1 Mechanical | pipe fittings (-2008049) | 3 | slot 7 or later (3) | slot 1 (3) |
| RE1 Plumbing | pipe fittings (-2008049) | 6 | slot 7 or later (6) | slot 1 (6) |
| RE1 Plumbing | pipes (-2008044) | 3 | before own (2), slot 4 (1) | slot 1 (3) |

- **The leading 3 is common, not constant.** It is the first slot of 79% of Core Interior's records and of most RE1 MEP records. RE1 Architecture's records never start with it; their first slot is a Level on 90 of 152.
- **The type has no fixed slot.** Within a category it is mostly in one place: slot 3 on 256 of Core Interior's 392 columns, slot 3 on all 13 of RE1's ducts, the slot before the record's own id on all 13 of RE1's furniture. Across categories it moves from slot 1 (walls, doors) to slot 3 or 4 (columns, ducts, pipes), to after slot 6 (fittings, whose lists run long), to before the record's own id, the slot RE-23 found holds a door's host wall.
- **The Level moves the same way:** slot 0 on RE1 Architecture, slot 1 on RE1's MEP curves and fittings and on Core Interior's rooms, slots 2 to 4 on Core Interior's walls, doors and columns. A column names two Levels, its base and its top (RE-59).
- **Most slots are other declared elements.** Across Core Interior's records, 146,929 slots hold ids the ElemTable declares that are neither the element's known type nor a Level. That count includes types whose element's type is not known, so it is not a count of unattributed references.

## 4. What this changes

Nothing in code. The slots behave as if the list follows the fields the element's class serializes: the slot of a type or a Level is steady within a category and moves between them. That reading is not proven here; the category stands in for the class. rvt-rs already reads both without a slot table: the type from the partition name entries (RE-38) and the Level by value, as the one Level, or the base of two, the record names (RE-27, RE-59). A per-class slot table would read the same values with less evidence behind each entry.

The leading 3 is left unattributed. It is absent from every RE1 Architecture record, where a Level takes the first slot, but present on the other RE1 models of the same release.

## 5. Reproduce

```text
gh workflow run measure.yml --ref re/re155-reference-slots -f base=none -f probe=probe_re155_reference_slots
```

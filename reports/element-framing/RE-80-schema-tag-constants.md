# RE-80 — Three per-release constants are class tags in the file's own schema

**Date:** 2026-09-27
**Issues:** #154, #223; Discussion #112
**Credit:** jakobhirn-bit (Discussion #112: the record marker is the ElementHeader tag in Formats/Latest, so a reader needs no version table). Measured here, it is two neighbouring class tags.
**Probe:** `examples/probe_re80_schema_tag_constants.rs`

**Result:** positive on every release from 2016 to 2026.

RE-76 established that a class's serialization tag is its definition ordinal in the file's own `Formats/Latest`. Three numbers rvt-rs has carried as per-release constants turn out to be such tags:

1. The `u16` that opens `Global/ElemTable`, read since M1 as `element_count` and known to be the same on every file of a release, is the tag of the class `ElemTable`. The stream starts with its own class.
2. The 8-byte element-record marker before the bounding box (RE-32) is `[tag of Outline][0xFF x 4][tag of ElementParents]`. The record writes its box as an `Outline` and then its counted reference lists (`+0x88`) as `ElementParents`. RE-32's rule, "the ElemTable constant plus 40", holds only because `ElementParents` has sat 40 definitions after `ElemTable` since 2016.
3. The `u16` in each record-chain header (RE-35, "the marker constant minus 12") is the tag of `ElementHeader`, on 2024, 2025 and 2026.

## Measured

On the 11-release family corpus (`phi-ag/rvt`), Core Interior (2024), RE1 Architecture (2025) and Snowdon Towers (2024, local only):

| file | ElemTable = header | Outline | ElementParents | `[Outline FFx4]` prefixes | ending in ElementParents, with a box | chain records opening with ElementHeader |
|---|---|---:|---:|---:|---:|---:|
| family 2016 | 1174 = 1174 | 266 | 1210 | 16 | 16 | 0 |
| family 2017 | 1191 = 1191 | 263 | 1227 | 16 | 16 | 0 |
| family 2018 | 1232 = 1232 | 265 | 1268 | 16 | 16 | 0 |
| family 2019 | 1233 = 1233 | 266 | 1273 | 16 | 16 | 0 |
| family 2020 | 1244 = 1244 | 269 | 1284 | 16 | 16 | 0 |
| family 2021 | 1271 = 1271 | 288 | 1311 | 16 | 16 | 0 |
| family 2022 | 1312 = 1312 | 297 | 1352 | 16 | 16 | 0 |
| family 2023 | 1370 = 1370 | 316 | 1410 | 32 | 32 | 0 |
| family 2024 | 1411 = 1411 | 326 | 1451 | 32 | 32 | 1,975 |
| family 2025 | 1451 = 1451 | 345 | 1491 | 32 | 32 | 1,984 |
| family 2026 | 1481 = 1481 | 353 | 1521 | 32 | 32 | 1,992 |
| Core Interior (2024) | 1411 = 1411 | 326 | 1451 | 23,815 | 23,815 | 28,560 |
| RE1 Architecture (2025) | 1451 = 1451 | 345 | 1491 | 1,562 | 1,562 | 4,672 |
| Snowdon Towers (2024) | 1411 = 1411 | 326 | 1451 | 33,611 | 33,611 | 47,634 |

"With a box" means six finite doubles follow, each minimum at most its maximum. Every `[Outline][0xFF x 4]` prefix in every file ends in the `ElementParents` tag.

On 2024 and 2025 the derived markers are exactly `BBOX_MARKER` and `BBOX_MARKER_2025`. Before 2024 no partition opens with a chain of RE-35's shape, which fits the different pre-2024 record envelope Steffen measured on 2015 and 2017 (Discussion #112).

## What it changes

- Nothing in the output. `bbox_marker` keeps its release table, and element records stay decoded on 2024 and 2025 only.
- A reader can derive the marker for any release from the schema, and every release from 2016 to 2026 frames element records behind it. Records on 2016 to 2023 and 2026 still need their prologue layout checked against a project and its export, which the reference pack (#407) does not yet hold for those releases (#421).
- `ElemTableHeader::element_count` and `bbox_marker` are documented as what they are.

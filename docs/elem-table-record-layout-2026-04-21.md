# Global/ElemTable record layout — 2026-04-21

Hex-dump-level RE of the `Global/ElemTable` decompressed body across
three corpus variants. Run via `examples/probe_elem_table_hex.rs`.

## Finding: record size varies by file variant

The existing `elem_table::parse_records_rough` assumes a 12-byte
record (three `u32` fields). That assumption holds on family files
but breaks on project files — they use wider records with explicit
per-record FF-marker prefixes.

| Variant | Decompressed size | Record start | Marker per record | Record size |
|---|---|---|---|---|
| Family (RAC 2024 sample) | 79,606 B | `0x30` | none (implicit) | **12 B** (current parser works) |
| Project 2023 (Einhoven) | 73,245 B | `0x1E` | `FF FF FF FF` (4 B) | **28 B** |
| Project 2024 (Core Interior) | 1,059,812 B | `0x22` | `FF FF FF FF FF FF FF FF` (8 B) | **40 B** |

Tangential header observation: `header_flag` = `0x0011` only on
family files. On both project files probed, the 16 bits at offsets
`0x1E` and `0x22` are either `0x0000` or inside the marker region,
so the header-flag heuristic in `parse_header` returns 0 on project
files. Not a parser bug — the flag genuinely isn't there.

## Hex evidence

### Family 2024 (`racbasicsamplefamily-2024.rfa`)

```
0x0000  83 05 b7 07 00 00 00 00 00 00 00 00 00 00 00 00
         └─ 0x0583 = 1411 (element_count)
               └─ 0x07B7 = 1975 (record_count)
0x0020  00 00 11 00 00 00 00 00 00 00 00 00 00 00 01 00
               └─ header_flag = 0x0011 at offset 0x22
0x0030  00 00 00 00 00 00 3f 00 00 00 3f 00 00 00 3f 00
         └─ record-area begins here
```

### Project 2023 (`Revit_IFC5_Einhoven.rvt`)

```
0x0000  5a 05 37 0a 00 00 00 00 00 00 00 00 00 00 00 00
         └─ 0x055A = 1370 (element_count)
               └─ 0x0A37 = 2615 (record_count)
0x0010  00 00 00 00 00 00 00 00 00 00 00 00 00 00 ff ff
                                                    ├─ record 0 marker
0x0020  ff ff 01 00 00 00 01 00 00 00 00 00 00 00 00 00
         ┘   └─ id_primary = 1       └─ id_secondary = 1
0x0030  00 00 00 00 00 00 00 00 00 00 ff ff ff ff 02 00
                                      └─ record 1 marker
0x0040  00 00 02 00 00 00 00 00 00 00 00 00 00 00 00 00
              └─ id_primary = 2
0x0050  00 00 00 00 00 00 ff ff ff ff 03 00 00 00 03 00
                          └─ record 2 marker
```

Records start at `0x1E`. Each record is 28 bytes:

```
offset +0  | FF FF FF FF                    (4-byte marker)
offset +4  | u32 id_primary   (monotonic: 1, 2, 3, …)
offset +8  | u32 id_secondary (matches id_primary on observed samples)
offset +12 | 16 bytes of payload (mostly zero on this sample)
```

### Project 2024 (`2024_Core_Interior.rvt`)

```
0x0000  83 05 39 67 00 00 00 00 00 00 00 00 00 00 00 00
         └─ 0x0583 = 1411 (element_count — same as family)
               └─ 0x6739 = 26,425 (record_count — 13× family scale)
0x0020  00 00 ff ff ff ff ff ff ff ff 00 00 00 00 01 00
               └─ record 0 marker (8 bytes of 0xFF)
                                                    └─ id_primary = 1
0x0040  00 00 01 00 00 00 00 00 00 00 ff ff ff ff ff ff
              └─ id_secondary = 1  └─ record 1 marker begins
0x0050  ff ff 00 00 00 00 02 00 00 00 00 00 00 00 00 00
              …payload…  └─ id_primary = 2
```

Records start at `0x22`. Each record is 40 bytes:

```
offset +0  | FF × 8                       (8-byte marker)
offset +8  | 4 bytes of zero (alignment?)
offset +12 | u32 id_primary  (monotonic: 1, 2, 3, …)
offset +16 | 12 bytes of payload/zero
offset +28 | u32 id_secondary (matches id_primary on observed samples)
offset +32 | 8 bytes of payload/zero
```

## Why the rough parser early-terminates on project files

`parse_records_rough` scans for a single `0xFFFFFFFF` as the record-
area trailer sentinel. On project files the marker appears AT THE
START of every record, not just once at the end. With the 2026-04-21
sentinel-start fix (scan from `0x30`, not `0`):

- Project 2023 never hits `0x30`; the first marker at `0x1E` is
  skipped by the `start` offset, but subsequent markers at `0x3A`,
  `0x56`, etc. get picked up — so records 2+ parse as "the sentinel",
  truncating at record #2.
- Project 2024 has markers at `0x22` (before `0x30`) and `0x4A`
  (after `0x30`). The first post-`0x30` marker at `0x4A` terminates
  the scan immediately → 2 records returned.

## Path to a correct project-file parser

The rough parser needs three things to work on real `.rvt` files:

1. **Detect the record size** by locating the first two
   consecutive markers and taking their spacing: 28 B (4-byte
   marker) or 40 B (8-byte marker).
2. **Start offset** = offset of the first marker, not hard-coded
   `0x30`. Family files end up at `0x30` because the marker is
   implicit; project files surface the first marker earlier.
3. **Termination** = after N records, where N is the header's
   `record_count` field. Each variant's record-count header value
   is accurate (1975 / 2615 / 26425), so walk exactly that many
   records and stop.

### Landed

`elem_table::parse_records(&mut RevitFile) -> Vec<ElemRecord>` (alongside
the pre-existing `parse_records_rough` for backward compat) implements
all three steps via a new `detect_layout()` scanner that finds the
first two FF markers and takes their stride. Verified against the
3-file corpus:

| Variant | Before (rough) | After (parse_records) |
|---|---|---|
| Family 2024 (`.rfa`) | 45 records (12 B implicit from 0x30) | 1975 records (uses header count) |
| Project 2023 (`.rvt`) | 2 records (sentinel early-term) | 2614 records (28 B from 0x1E) |
| Project 2024 (`.rvt`) | 2 records (sentinel early-term) | 26,425 records (40 B from 0x22) |

`tests/elem_table_corpus.rs` pins these counts in CI when the magnetar
corpus is present (skips gracefully when not). First 3 `id_primary`
values on both project files are `1, 2, 3` — sequential element ids,
exactly what the walker needs to index into `Global/Latest`.

This unblocks the record-enumeration half of walker → IFC emission.
The remaining half — binding `ElemRecord.id_primary` to a byte offset
inside `Global/Latest` — requires decoding the per-record payload
(16 B on 2023, 28 B on 2024). See "Remaining unknowns" below.

Remaining unknowns that the 3-file corpus can't yet answer:

- Is the 28 B → 40 B record-size shift a per-release change (2023
  vs 2024) or a per-project-size change (Einhoven is 913 KB,
  Core_Interior is 34 MB)?
- What are the 16/28 payload bytes? Offset within
  `Global/Latest`? Class-tag reference? Parent-element handle?
- Are there multiple record types packed in the same stream
  (header, element, group)?

Need more project-file samples to disambiguate. The 3-file corpus
establishes the shape but not the full semantics.

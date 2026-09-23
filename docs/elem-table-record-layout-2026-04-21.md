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
| Family (RAC 2024 sample) | 79,606 B | `0x30` | none (implicit) | **12 B** (current parser works) — *superseded 2026-09-22: families use the 40-byte project records; see the last section* |
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

Records start at `0x1E` — **four bytes before the marker**, corrected
2026-08-30, see "Where the record array starts" below. Each record is
40 bytes:

```
offset +0  | u32 zero                     (zero on all 26,425 records)
offset +4  | FF × 8                       (8-byte marker: two u32 owner
           |                               slots, 0xFFFFFFFF = "none")
offset +12 | 4 bytes of zero (alignment?)
offset +16 | u32 id_primary  (monotonic: 1, 2, 3, …)
offset +20 | 16 bytes of payload/zero
offset +36 | u32 id_secondary (matches id_primary on observed samples)
```

`rvt-elem-table --raw` on record 0 of `2024_Core_Interior.rvt` prints the
whole 40 bytes and the framing reads off the hex directly:

```
00000000 ffffffff ffffffff 00000000 01000000
000000000000000000000000 01000000
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
| Project 2024 (`.rvt`) | 2 records (sentinel early-term) | 26,425 records (40 B from 0x1E — this row read `0x22` and was one record short in practice until #206; see "Where the record array starts") |

`tests/elem_table_corpus.rs` pins these counts. It is named by the
`test` matrix job (family half, `RVT_REQUIRE_CORPUS=1`) and by
`corpus-tier2` (project half) — until 2026-08-30 it was named by no job
at all and skipped on every CI run, which is how the 2024 off-by-one
survived. First 3 `id_primary` values on both project files are
`1, 2, 3` — sequential element ids, exactly what the walker needs to
index into `Global/Latest`.

This unblocks the record-enumeration half of walker → IFC emission.
The remaining half — binding `ElemRecord.id_primary` to a byte offset
inside `Global/Latest` — requires decoding the per-record payload
(16 B on 2023, 28 B on 2024). See "Remaining unknowns" below.

Remaining unknowns that the 3-file corpus can't yet answer:

- Is the 28 B → 40 B record-size shift a per-release change (2023
  vs 2024) or a per-project-size change (Einhoven is 913 KB,
  Core_Interior is 34 MB)?
- **Payload bytes do NOT encode a byte offset into `Global/Latest`.**
  Follow-up probe (`examples/probe_elem_record_payload.rs`) dumped
  the 16 B / 28 B payload across all 2614 + 26,425 records on the
  two project files: they're predominantly zero on initial
  element-index records. The only bytes that look like in-range
  u32 offsets are incidental (3.6% of records at any given
  4-byte position — no better than random). This means
  `id_primary` → byte-offset binding must come from elsewhere:
  either a separate index stream we haven't located, or (more
  likely) a one-pass scan of `Global/Latest` where each element's
  self-id is read from a schema-described `m_id` field and
  captured into `HandleIndex`.
- The records near the END of the stream use a DIFFERENT field
  layout inside the same 40 B / 28 B stride (the `FF`×8 owner
  sentinel is replaced by real owner ids). Only 4,057 of the
  26,425 records on project 2024 still carry `FF`×8 at `+4`; the
  first without it is record 1283. These are probably
  type-definition or version-history records packed into the same
  stream. The stride itself does not change — see "Where the record
  array starts".
- Are there multiple record types packed in the same stream
  (header, element, group, type, deletion)?

Implication for the walker→IFC pipeline: ElemTable is the
**authoritative declared-ID set**, not the offset index. A
`HandleIndex` can validate coverage against ElemTable but must
derive offsets by decoding `Global/Latest` through the schema.
Need more project-file samples to disambiguate the trailer
region's semantics. The 3-file corpus establishes the shape but
not the full semantics.


## Where the record array starts (2026-08-30, #206)

`elem_table::parse_records` returned 26,424 of the declared 26,425
records on `2024_Core_Interior.rvt` (sha256 `c805df44…`). The count
disagreement was never a CI failure because no job ran
`tests/elem_table_corpus.rs` with a corpus, so the test skipped on every
run. Root cause, from the bytes:

**1. The decompressed length is authoritative and CRC-verified.**
`Global/ElemTable` is a checksum-paged stream
(`compression::is_checksum_paged_stream`), and the production path strips
the page trailers before inflating. On this file the stored stream is
125,892 B (one full 65,249 B page + a 60,643 B remainder), the stripped
stream is 125,539 B, and it inflates to **1,057,030 bytes** — the gzip
member's own CRC32 *and* `ISIZE` both agree with that length. Inflating
the *un*-stripped bytes yields 1,059,812 bytes and **fails the CRC**, so
the "26,425 records" recorded in the "Landed" table above was measured on
a corrupt inflate. The 2023 file (10,709 B stored, under one page) is
never stripped and inflates to 73,245 B, CRC-clean.

**2. 1,057,030 = `0x1E` + 26,425 × 40, exactly.**

| Candidate origin | Records that fit | Bytes left over |
|---|---|---|
| `0x22` (first `FF` run) | 26,424 | 36 — neither a record nor padding |
| `0x1E` (`0x22` − one `u32`) | **26,425** | **0** |

**3. The four bytes ahead of the marker are a record field, not slack.**
The `u32` at `0x1E + 40k` is zero for **all 26,425** values of `k`,
including the 26,425th slot that only exists under the corrected origin.
The last record (`k = 26424`) reads
`00000000 | 00020855 | 00000000 | 00000000 | 00000000 | ffffffff |
00000901 | 00020859 | 00000000 | 00000000` — a zero-id terminator, but a
structurally well-formed 40-byte record that ends flush with the stream.

So the marker is a **sentinel-valued field inside** record 0, not record
0's first byte. `detect_layout` took the first `0xFF` run as the origin,
which shifted every record window forward by four bytes and ran the walk
four bytes short of the last record. `elem_table::flush_origin` now
recovers the origin as `len − record_count × stride` whenever that lands a
whole number of `u32` fields (and less than one stride) ahead of the
marker, and `ElemTableLayout::marker_offset` records the difference so
field extraction stays anchored to the marker. Every id value on records
0…26,423 is byte-identical before and after; what changes is
`ElemRecord::offset` / `raw` (−4) and the recovery of record 26,424.

### The 2023 variant is deliberately left alone

The same end-anchoring on `Revit_IFC5_Einhoven.rvt` would put the origin
at `73,245 − 2615 × 28 = 25 = 0x19`, i.e. **five** bytes ahead of the
marker at `0x1E`. A record field cannot begin at a non-`u32` boundary
inside the record, so the `% 4` guard in `flush_origin` rejects it and
the 28-byte variant keeps the marker as its origin. That file therefore
still walks 2,614 of 2,615 records and leaves a 23-byte tail — an open
question (a short trailer record? a differently-framed final entry?), not
a silently-loosened assertion. `tests/elem_table_corpus.rs` pins 2,614
exactly so the day it changes, it changes visibly.

### Record-count semantics, as now understood

`record_count` (u16 LE at offset 2) is the number of fixed-stride records
in the array, **including** trailer/terminator records with `id_primary
== 0`; it is not a count of distinct ElementIds — that is
`element_count` (1,411 here, against 26,425 records). `declared_element_ids`
therefore includes `0` for this file after dedup.

## The sentinel field is not always `0xFF` (2026-09-22)

`detect_layout` measured the stride as the distance between the first two
`0xFF` sentinel runs. On Autodesk's Snowdon Towers 2024 architectural
sample (`Snowdon Towers Sample Architectural.rvt`, sha256 `33271010…`,
build `20230405_1134_fb02163c5b4b`) that distance is 120 bytes, not 40:
records 1 and 2 hold `0x10` in the field that is `FF`×8 on every other
record, so the first two runs are records 0 and 3. The stream is
1,889,350 bytes with `record_count` 47,233, and
`(1,889,350 − 30) / 40 = 47,233` exactly; the runs after record 3 are
40 bytes apart.

At 120 bytes the flush check failed, the marker became the origin, and the
walk read 15,744 windows whose `id_primary` was 0 on every one. The
declared ElementId set was `{0}`, so no partition element record could
join and the IFC export fell back to the 64-floor cap of the plan-loop
heuristic.

The detector now keeps the measured spacing when it tiles the stream flush
against `record_count`, and otherwise tries the known explicit strides (40,
then 28) that divide the spacing and do tile it. On that file this gives
stride 40, origin `0x1E`, marker offset 4 and 47,233 records (47,232
distinct non-zero ids plus the zero-id terminator). Every other corpus file
tiles at its measured spacing and detects exactly as before.

What the "sentinel" is: read as a `u64` at record `+4`, the field is an
ElementId whenever it is not `-1`. On `2024_Core_Interior.rvt` it is set on
22,368 of 26,425 records and every one of those values is an ElementId the
same table declares (20,522 of them lower than the record's own id); on the
Snowdon sample it is set on 29,177 of 47,233 and 29,131 are declared. The
`FF`×8 run the detector anchors on is therefore this field's *unset* value,
and a file whose first records all reference another element would have no
run in the scan window at all. RE-31
(`reports/element-framing/RE-31-elemtable-owner-field.md`) measures what
it names: 87 % of the set values on Core Interior also sit in the element's
own partition reference list, and by category they are the curtain wall of
a curtain panel, mullion or grid, the sketch of a sketch line, and the
model group of a grouped element. The library reads it as
`ElemRecord::owner_id`.

`record_count` is still read as a `u16`. No corpus file has more than
65,535 records, so whether bytes 4–5 hold the high half (that is, whether
the field is really a `u32`) is unmeasured. A file past that bound would
fail the flush check and keep the marker-spacing layout rather than parse
a wrong count.

## Family files use the project records (2026-09-22)

The "12-byte implicit layout from `0x30`" in the table above was wrong for
every family release. The parser read ids of 0, 4128768, 196608 and so on
from the 2026 family, and nothing checked them. phi-ag/rvt's README, which
reads a 2026 family ElemTable as 40-byte chunks after a 6-byte header,
prompted a re-measurement on all 11 `phi-ag/rvt` family releases:

- **Same records as projects.** Families use 28-byte records through
  2023 and 40-byte records from 2024, with record 0 at `0x1E`. From
  there, `id_primary` (at `+4` or `+16`) runs 1, 3, 4, 5, … and equals
  `id_secondary` (at `+8` or `+36`) on every record except the final
  zero-id terminator, which projects have too.
- **Why neither anchor applied.** Families leave the owner field (RE-31)
  at `0` where projects write `0xFF`, so there is no marker run to scan
  for. The record array also ends in a trailer (576 bytes on 2026, 427 on
  2016), so the flush check never applies. `detect_layout` now recognises
  these tables by that id progression. `record_count` records are parsed,
  and an owner of `0` reads as unset.
- **"header_flag".** The `0x0011` at `0x1E` (28-byte) or `0x22` (40-byte)
  is record 0's owner field, naming ElementId 17. It was never a header
  field.
- **"element_count".** The header `u16` read as `element_count` is not a
  count of elements. It is the same value on every file of a release,
  project or family: 1370 on 2023, 1411 on 2024, 1451 on 2025 and 1481 on
  2026.

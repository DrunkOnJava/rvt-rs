# RE-01 synthesis — where do real Revit element instances live?

**Date:** 2026-04-21
**Artifacts:**
- `Revit_IFC5_Einhoven.rvt` (SHA256 `d3a0c6d37d3f47a1726bc5aa7fe3880ed3c13bbe819b5e64680f6710b15aa948`) — Revit 2023, 913 KB
- `2024_Core_Interior.rvt` (SHA256 `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014`) — Revit 2024, 33 MB

**Stated goal:** `walker::scan_candidates` finds only HostObjAttr (1 of 405 schema classes) on real project files. Find where real element instance data actually lives so the scanner can be fixed.

**Safety posture:** Static-only. OLE2 compound + truncated-gzip streams, no code execution.

## Executive finding

**Element instance data is NOT in `Global/Latest`.** It lives in the `Partitions/N` streams, each of which contains many concatenated gzip chunks (33 chunks / 3 MB decompressed for Einhoven's `Partitions/0`). The current `walker::scan_candidates` scans the 884-KB decompressed `Global/Latest`, which mostly holds ADocument document-level metadata — no walls, no floors. That explains the 1/405 class hit rate.

## Facts (verified)

**F1** — Both project files have an ADocument with 13 fields decoded cleanly (0 diagnostics). Entry @ 0x1ee5 on 2023, @ 0x157e0 on 2024. *Source:* `examples/probe_adocument_trailing_ids.rs`.

**F2** — ADocument's `m_elemTable` field is a `FieldType::Pointer` containing `[u32; 2]`. On 2023 Einhoven the value is `(2097249, 49)` — `2097249 > file_size=884529`, so it is NOT a direct byte offset into `Global/Latest`. On 2024 the value is `(1, 0)`, equally non-pointer-like. *Source:* probe output.

**F3** — `Global/ElemTable` parses cleanly:
- Einhoven 2023: 1370 elements, 2615 records, 28-byte stride (FF×4 marker)
- 2024 Core Interior: 1411 elements, 26425 records, 40-byte stride (FF×8 marker)

**F4** — Every `ElemTable` record body (post-marker) is `[u32 id_primary][u32 id_secondary][N bytes of zeros]`. *No offset, no size, no class_tag.* The parser's `id_primary`/`id_secondary` both equal the sequential ElementId for rows 0..N. Later rows retain the same pattern. *Source:* `examples/probe_elem_table_records.rs`.

**F5** — Stream inventory on Einhoven 2023:

| Stream | Raw size | Decompressed |
|---|---:|---:|
| `Global/Latest` | 64,737 B | 884,529 B |
| `Global/ElemTable` | 10,709 B | — |
| `Global/PartitionTable` | 130 B | 87 B |
| `Partitions/0` | 464,357 B | **3,015,590 B (33 chunks)** |
| `Partitions/1` | 2,194 B | 1,203 B |
| `Partitions/2` | 7,906 B | 1,766 B |
| `Partitions/3` | 3,104 B | 949 B |
| `Partitions/4` | 5,298 B | 1,758 B |
| `Partitions/5` | 100,304 B | **587,060 B (10 chunks)** |
| `Partitions/6` | 291 B | 12 B |

On 2024 Core Interior: `Partitions/46` raw = 17,072,209 B (17 MB), `Partitions/53` raw = 5,692,783 B (5.7 MB). These dwarf `Global/Latest` (1 MB decompressed).

**F6** — `Global/PartitionTable` decompressed is 87 bytes. Contains only the ASCII string `"Workset1"` + some header bytes. It is NOT a per-element index. *Source:* `examples/probe_partitions.rs`.

**F7** — Partition streams are multi-chunk gzip. `inflate_at_auto` returns only the first chunk (~131 KB header-like data). `inflate_all_chunks` recovers all chunks. Einhoven's `Partitions/0` has 33 chunks totalling 3 MB.

**F8** — Each partition chunk has a consistent header prefix:
```
[u32 counter] [u32 unknown_a] [u32 size_field] [u32 unknown_b]
```
Counter increments monotonically across chunks. Multiple chunks often share the same `u32 counter` (e.g. 4 chunks start with `21 0e 00 00`), suggesting a single logical record can span multiple chunks.

**F9** — Scanning all 33 `Partitions/0` chunks for ASCII substrings `"Wall"`, `"Floor"`, `"Door"`, etc. returns **zero hits**. Class identification on the wire is not via ASCII names — it must be via numeric class tags from `Formats/Latest`.

## Hypotheses

**H1 (confidence 0.95)** — Element instance data lives in `Partitions/*` chunks, keyed by numeric class tags (not ASCII). Tags match the `Formats/Latest` schema's `tag` field. *Supports:* F5, F7, F8, F9. *Contradicts:* nothing observed.

**H2 (confidence 0.75)** — Each partition chunk's leading u32 is an ElementId. Chunks with the same leading u32 are multi-part records for a single element. *Supports:* F8 (identical leading IDs on consecutive chunks). *Open:* not yet confirmed against ElemTable's declared ids.

**H3 (confidence 0.6)** — `Global/Latest` holds document-level singletons (ADocument, Symbol definitions, Category table, Level, Grid, BasePoint) but NOT per-element instances. Explains why ADocument + Levels decode cleanly but Wall/Floor/Door don't.

**H4 (confidence 0.5)** — The `ElemTable` records store only `(id, id)` pairs because the actual element location is implicit: enumerate all partition chunks in order, and the Nth chunk's element corresponds to the Nth `ElemTable` row. Would need to test by counting partition chunks and comparing to `element_count` (1370 vs 33 chunks on Einhoven → doesn't match, so H4 is likely WRONG).

## Open questions

**Q1** — What exactly is the format of a partition chunk's body after the 16-byte header?

**Q2** — How is an ElementId mapped to a partition + chunk offset? (ElemTable doesn't store it; PartitionTable is too small; must be derivable from the chunk headers themselves.)

**Q3** — On 2024 Core Interior, partitions are numbered 46, 48, 51, 53, 55, 59, 61, 65 (non-contiguous). Is the partition number meaningful (e.g. category id)?

**Q4** — What is the schema-class-tag representation inside partition chunks? Bare u16 LE? Something else?

## Decisions

**D1** — Scope of the original RE-01..08 task sequence was too narrow. The real scope is partition-chunk record format reverse engineering, which is substantially more involved than I estimated. New tasks reflect this — see below.

**D2** — Static analysis only. No dynamic execution of Revit files needed; all work happens via rvt-rs library + custom probes on copies.

## Artifacts produced

- `examples/probe_adocument_trailing_ids.rs` — dumps ADocument Pointer + ElementId fields
- `examples/probe_elem_table_records.rs` — walks ElemTable records with u32-window analysis
- `examples/probe_partitions.rs` — dumps Global/PartitionTable + first chunk of each partition
- `examples/probe_partition_chunks.rs` — full multi-chunk inflate, ASCII class-name scan

## Revised task list

The original RE-01..08 decomposition assumed the element table was a blind-scan problem in Global/Latest. It's not. New / revised tasks should be:

- **RE-09** Probe partition-chunk header structure (16-byte prefix hypothesis)
- **RE-10** Correlate chunk leading-u32 values with ElemTable IDs
- **RE-11** Probe chunk-body format: does it begin with schema class tag u16 LE?
- **RE-12** Map ElementId → (partition_number, chunk_index, offset) empirically
- **RE-13** Cross-version: repeat RE-09..12 on 2024 file (different partition numbering)
- **RE-14** Document `Partitions/*` wire format in `docs/partition-streams-2026-04-22.md`
- **RE-15** Rewrite `walker::scan_candidates` (or add `walker::iter_partition_elements`) to scan partition chunks rather than Global/Latest
- **RE-16** Validation: scanner returns >10 distinct classes on Einhoven 2023

Old RE-02..08 are SUPERSEDED. The existing tasks #395..401 in the task list should be retired + replaced with the above.

## Confidence in synthesis

- High: F1–F9 all verified via working probes checked into `examples/`.
- Medium: H1–H3 follow from the facts but multi-chunk structure is not yet byte-exact RE'd.
- Low: H4 (already refuted), Q1–Q4 still open.

## Recommended next step

Start RE-09 — probe partition-chunk header structure across multiple chunks of Einhoven's `Partitions/0`. Goal: confirm the 16-byte header interpretation and find where per-record element data begins within each chunk.

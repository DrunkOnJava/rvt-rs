# Wave 2 — Checksum-paged stream framing contract (narrowed)

Date: 2026-08-29  
Issue: [#151](https://github.com/DrunkOnJava/rvt-rs/issues/151)  
Discussion: [#112](https://github.com/DrunkOnJava/rvt-rs/discussions/112)  
Credit: reported finding [@STE1200](https://github.com/STE1200) (Steffen / team)  
Judge: Wave 1 independent verdict **narrow**  
Wave: Worker A production wiring under the narrowed contract

## Status of claims

| Claim (as reported) | Independent status | Confidence |
|---|---|---|
| Full stored page = **65,249** = **64,896** payload + **353** trailer | Confirmed (reviter + helpers + synthetics) | High |
| Trailers must be stripped before RFC 1951 inflate | **Reproduced** on large multi-member **Partitions/\*** | High |
| Without strip, inflate can terminate cleanly while drifting | **Reproduced** on synthetic inject fixtures | High |
| `Formats/Latest` loses ~**48%** of schema | **Not reproduced** on redistributable corpus; `class_names` *drops* after naive strip | Low for the 48% figure |
| Affects streams ≳ **~190 KB** | Narrowed: first full page at 65,249 stored bytes | Medium |
| Gzip member trailer oracle 209/209 | Not independently re-run | n/a |

## Layer order

```
CFB stored bytes  (read_stream — identity; never strip here)
  → [iff is_checksum_paged_stream] strip full-page 353-byte tails only
       (keep short final page verbatim; 65_249 = 64_896 + 353)
  → optional fixed header (0 / 8 / Contents / partition ≤44)
  → truncated-gzip member(s)
  → RFC 1951 DEFLATE
```

## Production strip gate (narrowed)

`is_checksum_paged_stream` enables strip for:

- `Partitions/<id>`
- listed `Global/*` DB streams (`Latest`, `ElemTable`, `History`, `PartitionTable`,
  `ContentDocuments`, `DocumentIncrementTable`)

**Excluded by default:** `Formats/Latest` (and all metadata / preview streams).
Naive Formats strip regresses opportunistic `class_names` (e.g. Core Interior
9579→8575) while SchemaTable stays flat. Use `is_revit_paged_loader_candidate`
for research/probes that still want the broader reviter-aligned set.

Raw `inflate_at` / `inflate_all_chunks` **never** strip. Named wrappers
(`prepare_stream_for_inflate`, `inflate_stream_at`, `inflate_stream_auto`,
`inflate_all_chunks_for_stream`) consult the gate.

## Gated call sites

| Call site | Streams | API |
|---|---|---|
| `partition_*` / `object_graph` partitions | `Partitions/*` | `inflate_all_chunks_for_stream` |
| `object_graph` / `walker` / `ifc` Global | `Global/Latest` | `inflate_stream_auto` |
| `elem_table` | `Global/ElemTable` | `inflate_stream_at` |
| `rvt_analyze` PartitionTable / Global | listed Global | `inflate_stream_*` |
| `reader` / walker / ifc Formats | `Formats/Latest` | `inflate_stream_*` (**no-op strip**) |
| `writer::decompress_stream` | patch verify | bare `inflate_at` (writer emits strip-clean gzip) |

## Writer boundary

`read_stream` stays stored-byte accurate. Writer re-encodes with
`truncated_gzip_encode` / `_with_prefix8` (no paged trailers) until a paged
encoder exists (Worker D). Verification must not strip writer output.

## Acceptance oracles

- Synthetic inject→strip→round-trip (`tests/checksum_page_framing.rs` + unit tests)
- Bare inflate must **not** equal payload; gated Partitions/Global strip must
- Formats ungated: `prepare_stream_for_inflate("Formats/Latest", …)` is identity
- Redistributable large partitions: member ok-count gains (e.g. Core Interior
  `Partitions/46` 925→935) — directional, not a sacred fail-count
- Formats SchemaTable non-regression; **do not** use `class_names` as success

## Out of scope / not claimed

- Formats ~48% schema recovery
- Steffen 2014/2026 endpoint coverage without fixtures
- 209/209 gzip trailer oracle
- ECC validation of the 353-byte tail
- ElemTable ownership / geometry / #154 grammar
- Paged encoder for writer

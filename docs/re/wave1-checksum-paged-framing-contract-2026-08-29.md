# Wave 1 — Checksum-paged stream framing contract (draft for Wave 2)

Date: 2026-08-29  
Issue: [#151](https://github.com/DrunkOnJava/rvt-rs/issues/151)  
Discussion: [#112](https://github.com/DrunkOnJava/rvt-rs/discussions/112)  
Credit: reported finding @STE1200 (Steffen / team)  
Wave: Worker C investigation (probe / docs / contract). Judge: **narrow** (accept scoped path-gated contract).
Status note: PR #160 later landed path-gated `inflate_stream_*` call-site wiring on `main` — not a universal strip inside bare `inflate_at`.

## Status of claims

| Claim (as reported) | Independent status (this wave) | Confidence |
|---|---|---|
| Full stored page = **65,249** = **64,896** payload + **353** trailer | **Consistent** with ahzs645/reviter + helpers already on `main` | High |
| Trailers must be stripped before RFC 1951 inflate | **Independently reproduced** on large multi-member **Partitions/\*** streams | High |
| Without strip, inflate can terminate cleanly while drifting | **Reproduced** on synthetic injected fixtures; **observed** as silent success + wrong/short member sets on real partitions | High |
| `Formats/Latest` loses ~**48%** of schema | **Not reproduced** on redistributable corpus: structured `SchemaTable` class/field counts stay flat; opportunistic `class_names` *drop* after strip | Low for the 48% Formats figure on our samples |
| Affects streams ≳ **~190 KB** | **Narrowed**: first full page at 65,249 stored bytes; measurable chunk-oracle wins appear once multi-member partitions span many pages (e.g. 17 MB `Partitions/46`) | Medium (threshold is sample-/oracle-dependent) |
| Gzip member trailer oracle 209/209 | **Not independently re-run** (reporter fixture + trailer methodology not in-repo) | n/a |

**Verdict for Wave 2:** treat checksum-page stripping as a **real container-layer requirement** for paged database streams, gated by path. Do **not** claim a Formats/Latest “48% schema loss” fix until a Formats-specific content oracle diverges on a redistributable file.

## Entry-point map

`RevitFile::read_stream` returns **stored** CFB bytes (must stay identity-accurate for writer copies).

Helpers (since `b826514`, extended in #160):

- `REVIT_STORED_PAGE_BYTES` / `REVIT_PAGE_PAYLOAD_BYTES` / `REVIT_PAGE_CHECKSUM_BYTES`
- `is_checksum_paged_stream`, `strip_revit_page_checksums`, `prepare_stream_for_inflate`
- `inflate_stream_at` / `inflate_stream_auto` / `inflate_all_chunks_for_stream` (path-gated strip, then inflate)

**Wave 1 snapshot (pre-#160):** production callers used bare `inflate_at` / `inflate_all_chunks`.

**Post-#160 on `main` (narrow / path-gated):** named-stream call sites use `inflate_stream_*`. Bare `inflate_at` remains a raw codec (no silent strip).

| Call site | Stream(s) | API after #160 | Framing assumption |
|---|---|---|---|
| `reader::schema` / `schema_table` | `Formats/Latest` | `inflate_stream_at` | gzip @ 0, strip if paged |
| `walker` | Formats + Global/Latest | `inflate_stream_at` / auto | same |
| `ifc` | Formats + Global/Latest | `inflate_stream_at` / auto | same |
| `object_graph` | Global/Latest, Partitions | `inflate_all_chunks_for_stream` | multi-member; strip if paged |
| `partition_*` / `elem_table` | Partitions, ElemTable | `inflate_all_chunks_for_stream` / `inflate_stream_at` | same |
| `writer::decompress_stream` | patched streams | `inflate_stream_at` | strip if paged path |
| bins (`rvt-analyze`, `rvt-corpus`, …) | various | mix of stream-aware + bare | prefer stream-aware |

Path gate (matches reviter): `Formats/Latest`, listed `Global/*` database streams, `Partitions/<id>`. **Exclude** `BasicFileInfo`, `PartAtom`, `ProjectInformation`, previews.

## Framing contract (Wave 2 target — landed path-gated on `main` via #160)

### Layer order

```
CFB stored bytes
  → [if is_checksum_paged_stream] strip full-page 353-byte tails (keep short final page)
  → optional fixed header (0 / 8 / Contents / partition ≤44)
  → truncated-gzip member(s) (magic 1F 8B 08; usually no CRC32+ISIZE)
  → RFC 1951 DEFLATE body
```

### Rules

1. **Strip before inflate** on gated paths only. Never strip inside `read_stream`.
2. **Full pages only:** for each complete 65,249-byte page, keep `[0, 64_896)` and discard `[64_896, 65_249)`. Concatenate. Retain the final short page verbatim.
3. **Do not validate/repair** the 353-byte tail in Wave 2 (reviter also defers ECC). Optional later: fail-closed if a declared page length is truncated mid-page when a higher-level length oracle exists.
4. **Header preservation:** page stripping applies to the **entire stored stream** (headers are inside page payload), matching reviter/`OdBm` loader stack (`PagedStream` outside `FixedHeaderReader`).
5. **Multi-member partitions:** after strip, continue to discover members via gzip-magic scan (`inflate_all_chunks`). Primary acceptance oracle = successful member count + inflated totals improving or holding vs control on corpus partitions that previously had inflate failures.
6. **Formats/Latest:** still gzip @ 0 after strip. Acceptance must use **structured schema metrics** and/or synthetic round-trip — not raw inflate length alone (length can fall slightly while chunk oracles improve elsewhere).
7. **Writer:** re-encoding must emit stored pages **with** trailers if Revit readers require them, **or** document that writer output is strip-clean and only rvt-rs/reviter-class readers accept it. Wave 2 must not break `write_with_patches` identity on streams that were read as stored bytes. Prefer: strip on decode path only; writer continues to patch decompressed payloads and re-wrap with existing truncated-gzip encoders until a paged encoder exists.
8. **Fail-closed preference:** prefer explicit errors for truncated mid-page when we have an expected length; never “succeed” with drifted DEFLATE when strip was required and skipped.
9. **API shape (suggested):** keep `inflate_at` / `inflate_all_chunks` as raw codecs; add or finish thin wrappers (`inflate_stream_at` / `inflate_all_chunks_for_stream` / `prepare_stream_for_inflate`) and migrate named-stream call sites. Avoid silently stripping inside `inflate_at` (breaks tests that feed already-clean buffers).

### Acceptance tests for Wave 2

- [x] Synthetic inject→strip→round-trip (see `tests/checksum_page_framing.rs` + compression unit tests) stays green.
- [ ] On `2024_Core_Interior.rvt` `Partitions/46` (redistributable corpus): stripped path recovers **all** members that fail under control (Wave 1: control ~925 ok via `inflate_all_chunks`, stripped **935**; fail→0 under magic-scan oracle).
- [ ] `Formats/Latest` structured class/field counts do not regress vs control on Core Interior + Einhoven.
- [ ] Writer patch round-trip still passes for Formats/Global streams.
- [ ] Dated evidence table updated in reconnaissance / `reports/security/`.

## Out of scope for Wave 2

- Implementing ECC/checksum verification of the 353-byte tail
- Claiming Formats ~48% schema recovery without a Formats content oracle
- Changing CFB sector layout or OLE parsing

## Probes

- `examples/probe_checksum_page_evidence.rs` (Wave 1)
- `examples/probe_page_checksum_strip.rs` / `probe_schema_page_strip.rs` (pre-existing)
- Artifact dir: `/opt/cursor/artifacts/wave1_worker_c_decompress/`

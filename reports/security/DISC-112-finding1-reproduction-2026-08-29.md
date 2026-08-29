# Discussion #112 / issue #151 — Finding 1 independent reproduction (2026-08-29)

## Credit

Reported by [@STE1200](https://github.com/STE1200) / Steffen's team in
[Discussion #112](https://github.com/DrunkOnJava/rvt-rs/discussions/112).
Repository-side reproduction, regression tests, and production wiring by the
rvt-rs maintainers.

## Verdict: **REPRODUCED** (independently)

The reported checksum-paged framing is real on redistributable / local probe
corpora. Enabling page-trailer stripping before RFC 1951 inflate is required
for correct decode of streams that cross a full 65,249-byte stored page.

| Claim (reported) | Independent result |
|---|---|
| Full page = 64,896 payload + 353 trailer | Confirmed (constants + layout) |
| Drift / silent success without strip | Confirmed via gzip ISIZE trailer oracle and multi-chunk yield |
| ~48% Formats schema loss | **Not observed** as schema class-count loss on magnetar/project samples (class counts stay flat); trailer oracle still shows control inflate is wrong |
| Affects streams ≳190 KiB | Confirmed directionally; even single full-page streams (≳65 KiB stored) show trailer mismatch without strip |

## Evidence table (2026-08-29, local probe — not redistributed)

Source files were opened read-only. Hashes and aggregate metrics only; no
private stream payloads committed.

### `2024_Core_Interior.rvt` (magnetar-io project corpus)

| Stream | Stored | Full pages | Control inflate bytes / trailer ISIZE | Strip inflate bytes / trailer ISIZE | Notes |
|---|---:|---:|---|---|---|
| Formats/Latest | 173230 | 2 | 472791 / **fail** | 470502 / **ok** | Schema classes 395=395; control is longer but trailer-invalid |
| Global/Latest | 78810 | 1 | 1014963 / fail | 1008506 / ok | |
| Global/ElemTable | 125892 | 1 | 1059812 / fail | 1057030 / ok | |
| Global/ContentDocuments | 243084 | 3 | 1463023 / fail | 1456498 / ok | |
| Partitions/46 | 17072209 | 261 | 925 chunks / 97.96 MiB | **935 chunks / 98.77 MiB** | Multi-member yield improves with strip |

Artifacts: `/opt/cursor/artifacts/stream_evidence_2024_core_interior.json`,
`finding1_evidence_table.json`.

### Additional samples

| File | Provenance | Finding |
|---|---|---|
| `Revit_IFC5_Einhoven.rvt` | magnetar-io | Formats trailer fail→ok after strip; classes 405=405 |
| `bricks.rfa` | public Autodesk family fetch (2014-era) | Formats trailer fail→ok after strip |
| tier1 synthetic fixtures | Apache-2.0 in-repo | Below one page — strip is identity (no regression) |

### Synthetic regression

`compression::tests::synthetic_multipage_formats_requires_strip_before_inflate`
builds a high-entropy truncated-gzip payload, injects 353-byte trailers every
64,896 bytes, and asserts `inflate_stream_at("Formats/Latest", …)` recovers
the payload while bare `inflate_at` does not.

## Production change

- `prepare_stream_for_inflate` / `inflate_stream_at` / `inflate_stream_auto` /
  `inflate_all_chunks_for_stream` strip trailers for checksum-paged paths.
- Call sites (reader schema, walker, partitions, elem table, IFC, writer
  decompress, CLIs) use the stream-aware helpers.
- `RevitFile::read_stream` remains stored-byte accurate (writer identity copies).

## Confidence

**High** for the framing + trailer oracle + partition multi-chunk yield.
**Medium** for the reporter's exact "~48% Formats schema loss" magnitude —
not reproduced as class-count loss on available corpora; likely parser- or
fixture-specific. The silent-corruption hazard is still confirmed.

## Follow-ons (#152–#156)

Deferred per critical-path scope. Stream-evidence harness remains available
for ElemTable / element framing probes.

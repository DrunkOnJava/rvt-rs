# Discussion #112 — checksum-paged stream probe (2026-08-28)

## Claim

Discussion [#112](https://github.com/DrunkOnJava/rvt-rs/discussions/112) (Steffen / STE1200)
reports that Revit database streams are checksum-paged: every full stored page is
**65,249** bytes = **64,896** payload + **353** trailer, and trailers must be
stripped before DEFLATE. Without stripping, inflate can terminate cleanly while
drifting after the first page boundary (reporter: ~48% `Formats/Latest` loss on
their fixture). Independent public evidence: ahzs645/reviter
`docs/oda-loader-analysis.md` and `stripRevitPageChecksums` in
`lib/reviter/revit-container.ts` (same constants; ODA
`PagedStreamImplReader<..., 65249u>`).

## What we implemented

- `compression::REVIT_STORED_PAGE_BYTES` / `REVIT_PAGE_PAYLOAD_BYTES` /
  `REVIT_PAGE_CHECKSUM_BYTES`
- `is_checksum_paged_stream`, `strip_revit_page_checksums`,
  `prepare_stream_for_inflate`
- Unit tests for path gating and trailer cutting
- Examples: `probe_page_checksum_strip`, `probe_schema_page_strip`

`RevitFile::read_stream` remains **stored-byte accurate** so writer identity
copies are not corrupted.

## Private reproduction on redistributable corpus

| File | Stream | Raw len | Removed | Inflate raw | Inflate stripped | Schema classes | class_names raw→stripped |
|---|---|---:|---:|---:|---:|---:|---|
| Revit_IFC5_Einhoven.rvt | Formats/Latest | 169647 | 706 | 464741 | 462765 | 405=405 | 10495→8474 |
| 2024_Core_Interior.rvt | Formats/Latest | 173230 | 706 | 472791 | 470502 | 395=395 | 9579→8575 |
| 2024_Core_Interior.rvt | Global/Latest | 78810 | 353 | 1014963 | 1008506 | n/a | n/a |

On these files, naive page-aligned strip **does not** increase recoverd inflate
bytes; it slightly decreases them and reduces opportunistic `class_names` hits
while structured `SchemaTable` class counts stay flat. We therefore **did not**
enable strip on the default inflate path.

## Interpretation

1. The page size constants and public ODA/reviter evidence are credible.
2. Enabling a 0-aligned strip as the default on our current corpus is a
   **regression**, not a fix — page framing may start after a stream-specific
   header, use a different stride on some releases, or interact with multi-member
   gzip differently than the UNBC partition case.
3. Declaring a CVE/security advisory without the reporter's multi-page
   partition fixture (or another file where strip strictly improves oracle
   metrics) would be premature.

## Next steps

- Ask the reporter for a redistributable repro (or a minimized stream dump +
  expected inflated length / schema class count).
- Probe partition streams > ~190 KiB with gzip-member / ISIZE trailers as
  oracles (as the reporter did) before flipping the default.
- Keep `strip_revit_page_checksums` available for experimental probes and a
  future opt-in decode mode.


## Superseded by 2026-08-29 reproduction

See [`DISC-112-finding1-reproduction-2026-08-29.md`](DISC-112-finding1-reproduction-2026-08-29.md). Independent reproduction **confirmed** the page layout; production inflate now strips trailers. The 2026-08-28 note that strip was a Formats regression was based on inflate-length / class_name heuristics; the gzip ISIZE trailer oracle shows the shorter stripped output is the correct decode.

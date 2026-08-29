# Wave 2 — Writer audit for checksum-paged inflate (#151)

Date: 2026-08-29  
Credit: Finding reported by [@STE1200](https://github.com/STE1200)  
Judge verdict: **narrow** (`/opt/cursor/artifacts/wave1_judge_finding1/VERDICT.md`)  
Production wiring: #160 on `main`

## Verdict: **stored-accurate — PASS** (with documented limitation)

| Path | Policy | Status |
|---|---|---|
| `RevitFile::read_stream` | Identity stored CFB bytes | Unchanged; strip never runs here |
| Empty-patch `write_with_patches` | Byte-identical file copy | Pass (`empty_patch_round_trip_is_byte_identical_stored_accurate`) |
| Patch encode | `truncated_gzip_encode` / `_prefix8` (no 353-byte trailers) | Unchanged — **no paged encoder** in Wave 2 |
| `verify_patches_applied` / `decompress_stream` | Bare `inflate_at` on writer output | Restored in this support PR — must **not** use `inflate_stream_at` |
| Production readers | `inflate_stream_*` strip on gated paths | From #160; correct for authentic Revit paged streams |

## Why verify must stay bare-inflate

Writer-produced streams are strip-clean truncated-gzip. Applying
`strip_revit_page_checksums` to any buffer ≥ 65_249 bytes removes 353 bytes
at every page boundary **even when no trailers exist**. That false-strip
breaks WRT-13 verification for large Formats/Global patches.

Regression:
`writer::tests::large_formats_patch_round_trips_without_false_page_strip`.

## Known limitation (documented, not fixed here)

Re-reading a **writer-patched** paged-path stream (≥ one stored page) through
production `inflate_stream_*` will also false-strip until a paged encoder
exists. Empty-patch / unpatched stream copies remain safe (original trailers
preserved). Prefer docs + tests over encoder work per Wave 2 support scope.

## Acceptance oracles used

- Synthetic framing fixtures: `tests/checksum_page_framing.rs`
- Writer unit tests above
- Evidence matrix: `/opt/cursor/artifacts/wave2_evidence_matrix/`

# Stream-evidence harness (Discussion #112 / issue #151)

Reusable **control vs experimental** page-stripping evidence tool for
checksum-paged Revit CFB streams. Finding reported by
[@STE1200](https://github.com/STE1200) (Steffen) in
[Discussion #112](https://github.com/DrunkOnJava/rvt-rs/discussions/112);
tracked as [#151](https://github.com/DrunkOnJava/rvt-rs/issues/151).

This harness **does not change production inflate**. Experimental stripping
lives in `tools/stream_evidence` so probes can A/B compare without wiring
production readers.

## Package

| Path | Role |
|------|------|
| `tools/stream_evidence/` | Library + `stream-evidence` CLI |
| `examples/stream_evidence.rs` | Discoverability shim → same CLI |
| `docs/recon/stream-evidence-harness.md` | This note |

## Reported layout (hypothesis under test)

Each full stored page = **64,896** payload bytes + **353** checksum/ECC
trailer (**65,249** total). Short final pages keep all remaining bytes.
Tails must be removed before RFC 1951 inflate; leaving them in place can
terminate cleanly while drifting after the first page boundary.

## Run

```bash
cargo run -p stream-evidence --release -- \
  --file corpus/tier1/architectural-2024/architectural-2024.rvt \
  --stream Formats/Latest \
  -o /tmp/tier1_formats.json

# Project corpus (local only; not redistributed):
cargo run -p stream-evidence --release -- \
  --file _project_corpus/Revit/2024_Core_Interior.rvt \
  --stream Formats/Latest \
  -o /tmp/core_interior_formats.json
```

Omit `--stream` to analyze every suspected checksum-paged non-empty stream
(`Formats/Latest`, `Global/*` table streams, `Partitions/N`).

## JSON fields

Top-level `EvidenceReport`:

- `file`: `release`, `file_type`, `sample_hash_sha256`, `provenance`, `credit`
- `streams[]`:
  - `stream_name`, `page_layout` (stored lengths, suspected page counts /
    boundaries, per-page payload + tail lengths)
  - `control` / `experimental` arms: prepared lengths, gzip offsets,
    per-member decompressed lengths, consumed-byte counts, trailer results,
    parser results (`Formats/Latest` schema summary), failure offsets
  - `comparison`: equality, length delta, first divergence offset
  - `production_strip_matches_experimental`: whether
    `rvt::compression::strip_revit_page_checksums` matches the harness strip

## Library use from probes

```rust
use stream_evidence::{
    analyze_file, analyze_stored_stream, experimental_strip_page_checksums,
};

let report = analyze_file(path, Some(&["Formats/Latest".into()]), false, false)?;
let stream = analyze_stored_stream("Formats/Latest", &stored_bytes);
let stripped = experimental_strip_page_checksums(&stored_bytes);
```

## Provenance / clean-room

- Synthetic tier1 fixtures: license-free `gen-fixture` output under
  `corpus/tier1/`.
- External corpora (`_project_corpus/`, `_corpus/`): probe locally only;
  do not commit Autodesk sample bytes (see `SECURITY.md`).
- No production `src/compression.rs` behavior is changed by this crate.

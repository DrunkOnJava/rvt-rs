# Wave 2 evidence matrix — checksum-paged streams (#151)

Date: 2026-08-29  
Harness: `stream-evidence` (#158 on `main`)  
Credit: [@STE1200](https://github.com/STE1200)  
Judge: **narrow** — do not claim Formats ~48% schema recovery; pin concrete
file+stream; prefer SchemaTable / member-ok oracles over `class_names`.

## Corpus (redistributable / in-repo)

| Sample | Provenance | sha256 |
|---|---|---|
| `2024_Core_Interior.rvt` | magnetar-io MIT project corpus | `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014` |
| `Revit_IFC5_Einhoven.rvt` | magnetar-io MIT | `d3a0c6d37d3f47a1726bc5aa7fe3880ed3c13bbe819b5e64680f6710b15aa948` |
| `empty.rfa` | in-repo `tests/fixtures/families/` | `5f194a9a70a2ec1490bbdc61da03f63884da97e073917b6f096c1a198d8afd90` |
| tier1 synthetics | `corpus/tier1/*` (gen-fixture) | below one page — strip identity |

Raw JSON reports live under `/opt/cursor/artifacts/wave2_evidence_matrix/`.
This note summarizes directional results; do not overfit absolute fail counts
across oracles (judge note: magic-scan vs `inflate_all_chunks`).

## How to regenerate

```bash
mkdir -p /opt/cursor/artifacts/wave2_evidence_matrix
cargo run -p stream-evidence --release -- \
  --file _project_corpus/Revit/2024_Core_Interior.rvt --all-paged \
  -o /opt/cursor/artifacts/wave2_evidence_matrix/core_interior_all_paged.json
```

## Summary table (fill from harness run)

See `MATRIX.md` beside the JSON artifacts after the Wave 2 support run.

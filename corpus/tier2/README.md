# Tier-two corpus health (optional / external)

Tier two exercises **real project `.rvt` files** that are not redistributed
in this repository. Point `RVT_PROJECT_CORPUS_DIR` at a directory of `.rvt`
files (flat layout — one directory, not nested).

## Recommended external sources

| Source | License | Notes |
|--------|---------|-------|
| [`magnetar-io/revit-test-datasets`](https://github.com/magnetar-io/revit-test-datasets) `Revit/` | MIT | Used by CI Tier-two; known-count manifests under `tests/fixtures/project-counts/` for `2024_Core_Interior.rvt` and `Revit_IFC5_Einhoven.rvt` |
| Permissive clones via [`tools/fetch-corpus.sh`](../../tools/fetch-corpus.sh) | varies | Candidate hunt only — inventory with `tools/corpus-health.sh` |

**Do not** commit Autodesk-owned samples (see `SECURITY.md`).

## Local setup

```bash
git clone --depth 1 https://github.com/magnetar-io/revit-test-datasets _project_corpus
git -C _project_corpus lfs pull

export RVT_PROJECT_CORPUS_DIR="$PWD/_project_corpus/Revit"
cargo test --release --test corpus_tier2_health -- --nocapture
cargo test --release --test project_corpus_smoke -- --nocapture
cargo test --release --test project_count_fixtures -- --nocapture
tools/corpus-health.sh "$RVT_PROJECT_CORPUS_DIR"
```

Without `RVT_PROJECT_CORPUS_DIR`, Tier-two tests **skip gracefully** (not a
failure). CI always sets the env after cloning magnetar.

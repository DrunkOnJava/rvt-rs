# Tier-two corpus health (optional / external)

Tier two exercises **real project `.rvt` files** that are not redistributed
in this repository. Point `RVT_PROJECT_CORPUS_DIR` at a directory of `.rvt`
files (flat layout — one directory, not nested).

## Recommended external sources

| Source | License | Notes |
|--------|---------|-------|
| [`magnetar-io/revit-test-datasets`](https://github.com/magnetar-io/revit-test-datasets) `Revit/` | MIT | Used by CI Tier-two; known-count manifests under `tests/fixtures/project-counts/` for `2024_Core_Interior.rvt` and `Revit_IFC5_Einhoven.rvt` |
| Permissive clones via [`tools/fetch-corpus.sh`](../../tools/fetch-corpus.sh) | varies | Candidate hunt only — inventory with `tools/corpus-health.sh` |
| BIM4LCA / Nordic LCA (VTT, Granlund, Gravicon, Insinööritoimisto Kallinen) | CC BY-SA 4.0 | Fetch by URL, never committed. `ARK_NordicLCA_Housing_Concrete_BuildingPermit.rvt` (Revit 2024, 99.8 MB, sha256 `c6b53726a2d064654bfd9ebaa769bea5b85d8258ca980294a7f528bb239564e8`) from `ARCH.zip` (sha256 `025e33fb0c0ce3b7a869f4177a0862918cc987408f4cd34e58a0eabd8d9917c4`) listed at <https://cris.vtt.fi/en/publications/bim-files-to-download/>. Put it in the folder `RVT_NORDIC_LCA_DIR` points at, for `tests/column_node_z.rs`. Also `ARK_NordicLCA_Office_Concrete_As-Built.rvt` (Revit 2024, 64.6 MB, sha256 `747120005b4b35c7cb66ec61158aab50ded95e052e172cb387b86cd15b31d906`) from the same `ARCH.zip`, for `tests/column_cut_body.rs`. Attribution: BIM4LCA, CC BY-SA 4.0. |

**Do not** commit Autodesk-owned samples (see `SECURITY.md`).

## Local setup

```bash
git clone --depth 1 https://github.com/magnetar-io/revit-test-datasets _project_corpus
git -C _project_corpus lfs pull

export RVT_PROJECT_CORPUS_DIR="$PWD/_project_corpus/Revit"
cargo test --release --test integration -- corpus_tier2_health:: --nocapture
cargo test --release --test integration -- project_corpus_smoke:: --nocapture
cargo test --release --test integration -- project_count_fixtures:: --nocapture
tools/corpus-health.sh "$RVT_PROJECT_CORPUS_DIR"
```

Without `RVT_PROJECT_CORPUS_DIR`, Tier-two tests **skip gracefully** (not a
failure). CI always sets the env after cloning magnetar.

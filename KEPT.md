# Measure run 42027776156: kept held-out and rac_advanced counts and digests

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/42027776156, whose head side measured `04d1b101a30ffb5ddef8d0d37fbffea7b840b7a1`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 49 files for 24 models
- `base/`, from `measure-base`: 49 files for 24 models

Left out: everything but each held-out and `rac_advanced` model's `counts.txt` and `ifc.sha256`, the summary counts and export digests Measure keeps of them (Griffin, 2026-11-27, #2148), and `commit.txt`; the BIMcollab folders; and every other folder, whose outputs `refs/evidence/42027776156` keeps.

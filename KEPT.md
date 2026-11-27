# Measure run 41741114199: kept held-out and rac_advanced counts and digests

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/41741114199, whose head side measured `46da221fc89f259d7dd535e06e8f49a7ac0286a3`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 37 files for 18 models
- `base/`, from `measure-base`: 37 files for 18 models

Left out: everything but each held-out and `rac_advanced` model's `counts.txt` and `ifc.sha256`, the summary counts and export digests Measure keeps of them (Griffin, 2026-11-27, #2148), and `commit.txt`; the BIMcollab folders; and every other folder, whose outputs `refs/evidence/41741114199` keeps; the bimcollab models' folders, whose outputs Measure keeps to summary counts.

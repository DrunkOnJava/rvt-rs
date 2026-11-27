# Measure run 41707934595: kept held-out and rac_advanced counts and digests

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/41707934595, whose head side measured `01b5af37f7e7d186ac7b60e4c49b85ffdab36dd6`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 11 files for 5 models
- `base/`, from `measure-base`: 11 files for 5 models

Left out: everything but each held-out and `rac_advanced` model's `counts.txt` and `ifc.sha256`, the summary counts and export digests Measure keeps of them (Griffin, 2026-11-27, #2148), and `commit.txt`; the BIMcollab folders; and every other folder, whose outputs `refs/evidence/41707934595` keeps.

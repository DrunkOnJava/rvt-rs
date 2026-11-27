# Measure run 42017400629: kept held-out and rac_advanced counts and digests

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/42017400629, whose head side measured `5008247de01f35a305d74989992319786776f09e`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 49 files for 24 models
- `base/`, from `measure-base`: 49 files for 24 models

Left out: everything but each held-out and `rac_advanced` model's `counts.txt` and `ifc.sha256`, the summary counts and export digests Measure keeps of them (Griffin, 2026-11-27, #2148), and `commit.txt`; the BIMcollab folders; and every other folder, whose outputs `refs/evidence/42017400629` keeps.

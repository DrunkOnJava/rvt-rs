# Measure run 41741114199: kept held-out counts

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/41741114199, whose head side measured `46da221fc89f259d7dd535e06e8f49a7ac0286a3`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 19 files for 18 models
- `base/`, from `measure-base`: 19 files for 18 models

Left out: everything but each held-out model's `counts.txt`, the summary counts Measure keeps of it (Griffin, 2026-11-22), and `commit.txt`; the held-out folders' other files; and every other folder, whose outputs `refs/evidence/41741114199` keeps; the bimcollab models' folders, whose outputs Measure keeps to summary counts.

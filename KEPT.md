# Measure run 41707934595: kept held-out counts

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/41707934595, whose head side measured `01b5af37f7e7d186ac7b60e4c49b85ffdab36dd6`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 6 files for 5 models
- `base/`, from `measure-base`: 6 files for 5 models

Left out: everything but each held-out model's `counts.txt`, the summary counts Measure keeps of it (Griffin, 2026-11-22), and `commit.txt`; the held-out folders' other files; and every other folder, whose outputs `refs/evidence/41707934595` keeps.

# Measure run 42003913780: kept held-out counts

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/42003913780, whose head side measured `bcb021076bae0df45107cf8756cdbe588e0fb42a`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 19 files for 18 models
- `base/`, from `measure-base`: 19 files for 18 models

Left out: everything but each held-out model's `counts.txt`, the summary counts Measure keeps of it (Griffin, 2026-11-22), and `commit.txt`; the held-out folders' other files; and every other folder, whose outputs `refs/evidence/42003913780` keeps; the racadv models' folders, whose outputs Measure keeps to summary counts.

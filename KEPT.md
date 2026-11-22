# Measure run 41757734285: kept held-out counts

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/41757734285, whose head side measured `c53dc6844ba88d89207d0347e7971b792f2d2e25`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 19 files for 18 models
- `base/`, from `measure-base`: 19 files for 18 models

Left out: everything but each held-out model's `counts.txt`, the summary counts Measure keeps of it (Griffin, 2026-11-22), and `commit.txt`; the held-out folders' other files; and every other folder, whose outputs `refs/evidence/41757734285` keeps.

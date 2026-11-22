# Measure run 41658225355: kept held-out counts

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/41658225355, whose head side measured `2e07e27a3a90f579bc3bb0b76e86fa0856ea10f2`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 7 files for 6 models

Left out: everything but each held-out model's `counts.txt`, the summary counts Measure keeps of it (Griffin, 2026-11-22), and `commit.txt`; the held-out folders' other files; and every other folder, whose outputs `refs/evidence/41658225355` keeps.

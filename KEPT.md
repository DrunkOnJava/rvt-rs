# Measure run 41713761592: kept held-out counts

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/41713761592, whose head side measured `8fc583a165267ef842138d299472e7daecdd2bfe`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 2 files for 1 model

Left out: everything but each held-out model's `counts.txt`, the summary counts Measure keeps of it (Griffin, 2026-11-22), and `commit.txt`; the held-out folders' other files; and every other folder, whose outputs `refs/evidence/41713761592` keeps.

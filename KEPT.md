# Measure run 41741114199: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/41741114199, whose head side measured `46da221fc89f259d7dd535e06e8f49a7ac0286a3`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 1789 files for 93 models
- `base/`, from `measure-base`: 1789 files for 93 models
- `compare/`, from `compare-scorer-diff`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output; the 1 section of `scorer-diff.txt` that diff a left-out folder; the bimcollab models' folders, whose outputs Measure keeps to summary counts; the holdout models' folders, whose outputs Measure keeps to summary counts.

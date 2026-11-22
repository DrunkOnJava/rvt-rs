# Measure run 41757734285: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/41757734285, whose head side measured `c53dc6844ba88d89207d0347e7971b792f2d2e25`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 1789 files for 93 models
- `base/`, from `measure-base`: 1789 files for 93 models
- `compare/`, from `compare-scorer-diff`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output; the 1 section of `scorer-diff.txt` that diff a left-out folder; the holdout models' folders, whose outputs Measure keeps to summary counts.

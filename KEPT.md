# Measure run 42002548557: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/42002548557, whose head side measured `f3c87e4824c9399447da5d25cca55ac60c86ac78`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 1915 files for 93 models
- `base/`, from `measure-base`: 1915 files for 93 models
- `compare/`, from `compare-scorer-diff`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output; the holdout models' folders, whose outputs Measure keeps to summary counts.

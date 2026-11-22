# Measure run 42002553349: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/42002553349, whose head side measured `f3c87e4824c9399447da5d25cca55ac60c86ac78`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 13 files for 2 models
- `base/`, from `measure-base`: 13 files for 2 models
- `compare/`, from `compare-scorer-diff`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output.

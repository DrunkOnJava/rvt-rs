# Measure run 43662654246: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/43662654246, whose head side measured `b19b321cb4a14132c6a228193cdc0e7c8f941224`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 2559 files for 102 models
- `base/`, from `measure-base`: 2557 files for 102 models
- `verdict/`, from `measure-verdict`: 1 file
- `compare/`, from `compare-scorer-diff`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output; the holdout models' folders, whose outputs Measure keeps to summary counts.

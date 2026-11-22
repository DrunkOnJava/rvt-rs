# Measure run 42135914864: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/42135914864, whose head side measured `4ca938ad01041b5bb88da04dedc07304a67409bb`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 2146 files for 93 models
- `base/`, from `measure-base`: 2144 files for 93 models
- `compare/`, from `compare-scorer-diff`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output; the holdout models' folders, whose outputs Measure keeps to summary counts.

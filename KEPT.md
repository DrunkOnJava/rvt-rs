# Measure run 42091457764: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/42091457764, whose head side measured `830a4d941dcfda0a8a90444a3605e469ef2eff07`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 2018 files for 93 models
- `base/`, from `measure-base`: 2017 files for 93 models
- `compare/`, from `compare-scorer-diff`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output; the holdout models' folders, whose outputs Measure keeps to summary counts.

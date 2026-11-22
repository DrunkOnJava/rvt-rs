# Measure run 41973316830: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/41973316830, whose head side measured `3ad0762f6a8e1141e8e27cd1d940a24fcd0fc7a3`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 73 files for 4 models
- `base/`, from `measure-base`: 73 files for 4 models
- `compare/`, from `compare-scorer-diff`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output.

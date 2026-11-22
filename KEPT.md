# Measure run 43152271314: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/43152271314, whose head side measured `1d76c82fede5ee227ecc7d713f39e68abb226c3e`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 347 files for 76 models
- `base/`, from `measure-base`: 345 files for 76 models
- `compare/`, from `compare-scorer-diff`: 1 file
- `timing/`, from `measure-timing`: 2 files

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output.

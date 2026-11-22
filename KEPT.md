# Measure run 41656599871: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/41656599871, whose head side measured `05c957ae20587599be10d3f91ec95305bf66085e`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 1360 files for 32 models
- `base/`, from `measure-base`: 1360 files for 32 models
- `compare/`, from `compare-scorer-diff`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output; the holdout models' folders, whose outputs Measure keeps to summary counts.

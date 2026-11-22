# Measure run 41884639625: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/41884639625, whose head side measured `34750454de70c03783b4d766255e3952ede175ed`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 1590 files for 35 models
- `base/`, from `measure-base`: 1590 files for 35 models
- `compare/`, from `compare-scorer-diff`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output; the holdout models' folders, whose outputs Measure keeps to summary counts.

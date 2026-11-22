# Measure run 41884073888: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/41884073888, whose head side measured `34750454de70c03783b4d766255e3952ede175ed`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 69 files for 34 models
- `base/`, from `measure-base`: 69 files for 34 models
- `compare/`, from `compare-scorer-diff`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output.

# Measure run 46062940579: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/46062940579, whose head side measured `747193700a90ccf76840b1ae813cd09687f3fa2d`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 52 files for 10 models
- `base/`, from `measure-base`: 51 files for 10 models
- `verdict/`, from `measure-verdict`: 1 file
- `compare/`, from `compare-scorer-diff`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output.

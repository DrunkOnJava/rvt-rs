# Measure run 40156315852: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/40156315852, whose head side measured `9fcf08d3910cd5a53e95270b34c1018a9aeb7f13`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 1027 files for 31 models
- `base/`, from `measure-base`: 1027 files for 31 models

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output.

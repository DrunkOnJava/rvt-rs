# Measure run 45997957951: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/45997957951, whose head side measured `739406904c3a5cfc122b4e9d2d95fa5882c32c1a`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 466 files for 101 models
- `verdict/`, from `measure-verdict`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output.

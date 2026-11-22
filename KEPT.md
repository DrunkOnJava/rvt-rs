# Measure run 42106436497: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/42106436497, whose head side measured `b2b5716ed3e05c7b5050a6351c7da18135330600`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 292 files for 58 models

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output.

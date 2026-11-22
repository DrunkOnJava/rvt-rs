# Measure run 41658225355: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/41658225355, whose head side measured `2e07e27a3a90f579bc3bb0b76e86fa0856ea10f2`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 2 files for 0 models

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output; the holdout models' folders, whose outputs Measure keeps to summary counts.

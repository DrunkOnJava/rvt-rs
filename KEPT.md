# Measure run 42066757105: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/42066757105, whose head side measured `eb6de281f64761045ef01fdae6026c0a0b522b0b`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 17 files for 4 models

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output; the holdout models' folders, whose outputs Measure keeps to summary counts.

# Measure run 43688985749: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/43688985749, whose head side measured `1be73ffba9c1edf4622533e4f9ba4b2a105d34a7`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 71 files for 14 models
- `verdict/`, from `measure-verdict`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output; the holdout models' folders, whose outputs Measure keeps to summary counts.

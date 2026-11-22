# Measure run 41764492879: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/41764492879, whose head side measured `9e05f17900e27803848b03c6f5bfff570d379ab8`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 1822 files for 93 models
- `base/`, from `measure-base`: 1822 files for 93 models
- `compare/`, from `compare-scorer-diff`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output; the holdout models' folders, whose outputs Measure keeps to summary counts.

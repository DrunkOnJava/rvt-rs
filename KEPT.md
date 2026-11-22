# Measure run 41784044966: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/41784044966, whose head side measured `ecebf060a9c8684edc9c931ae80974036786d760`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 1822 files for 93 models
- `base/`, from `measure-base`: 1822 files for 93 models
- `compare/`, from `compare-scorer-diff`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output; the holdout models' folders, whose outputs Measure keeps to summary counts.

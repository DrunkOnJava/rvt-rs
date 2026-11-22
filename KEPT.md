# Measure run 42554270370: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/42554270370, whose head side measured `584c8a89315eef2b45ffdcdf55e74104f76623e0`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 2152 files for 93 models
- `base/`, from `measure-base`: 2150 files for 93 models
- `compare/`, from `compare-scorer-diff`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output; the bimcollab models' folders, whose outputs Measure keeps to summary counts; the holdout models' folders, whose outputs Measure keeps to summary counts; the racadv models' folders, whose outputs Measure keeps to summary counts.

# Measure run 41213779326: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/41213779326, whose head side measured `f9f71e688c36691d6bc51dd9c5d1fe589ac583ef`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 1360 files for 32 models

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output; the bimcollab models' folders, whose outputs Measure keeps to summary counts; the holdout models' folders, whose outputs Measure keeps to summary counts.

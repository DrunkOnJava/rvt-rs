# Measure run 42097940515: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/42097940515, whose head side measured `3258684780005b90efdcb242b277fbe1dc4676b1`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 326 files for 72 models

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output; the bimcollab models' folders, whose outputs Measure keeps to summary counts; the holdout models' folders, whose outputs Measure keeps to summary counts; the racadv models' folders, whose outputs Measure keeps to summary counts.

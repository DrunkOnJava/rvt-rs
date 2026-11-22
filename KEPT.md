# Measure run 42581222440: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/42581222440, whose head side measured `6f71f93a8dee7ad853926f9268bab32706996fce`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 2152 files for 93 models
- `base/`, from `measure-base`: 2150 files for 93 models
- `compare/`, from `compare-scorer-diff`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output; the 7 sections of `scorer-diff.txt` that diff a left-out folder; the bimcollab models' folders, whose outputs Measure keeps to summary counts; the holdout models' folders, whose outputs Measure keeps to summary counts; the racadv models' folders, whose outputs Measure keeps to summary counts.

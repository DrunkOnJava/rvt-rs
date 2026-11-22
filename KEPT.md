# Measure run 42027776156: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/42027776156, whose head side measured `04d1b101a30ffb5ddef8d0d37fbffea7b840b7a1`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 2009 files for 93 models
- `base/`, from `measure-base`: 2008 files for 93 models
- `compare/`, from `compare-scorer-diff`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output; the 24 sections of `scorer-diff.txt` that diff a left-out folder; the holdout models' folders, whose outputs Measure keeps to summary counts; the racadv models' folders, whose outputs Measure keeps to summary counts.

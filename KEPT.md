# Measure run 41707934595: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/41707934595, whose head side measured `01b5af37f7e7d186ac7b60e4c49b85ffdab36dd6`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 46 files for 1 model
- `base/`, from `measure-base`: 46 files for 1 model
- `compare/`, from `compare-scorer-diff`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output; the 1 section of `scorer-diff.txt` that diff a left-out folder; the holdout models' folders, whose outputs Measure keeps to summary counts.

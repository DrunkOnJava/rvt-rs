# Measure run 42003913780: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/42003913780, whose head side measured `bcb021076bae0df45107cf8756cdbe588e0fb42a`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 1915 files for 93 models
- `base/`, from `measure-base`: 1915 files for 93 models
- `compare/`, from `compare-scorer-diff`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output; the 7 sections of `scorer-diff.txt` that diff a left-out folder; the holdout models' folders, whose outputs Measure keeps to summary counts; the racadv models' folders, whose outputs Measure keeps to summary counts.

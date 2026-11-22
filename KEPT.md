# Measure run 42092391898: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/42092391898, whose head side measured `b452f96708aa3564b5376c550b3bd32e198513f5`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 53 files for 4 models
- `base/`, from `measure-base`: 49 files for 4 models
- `compare/`, from `compare-scorer-diff`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output; the 18 sections of `scorer-diff.txt` that diff a left-out folder; the holdout models' folders, whose outputs Measure keeps to summary counts.

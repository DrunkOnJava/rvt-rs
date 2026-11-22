# Measure run 42017400629: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/42017400629, whose head side measured `5008247de01f35a305d74989992319786776f09e`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 1916 files for 93 models
- `base/`, from `measure-base`: 1915 files for 93 models
- `compare/`, from `compare-scorer-diff`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output; the 7 sections of `scorer-diff.txt` that diff a left-out folder; the holdout models' folders, whose outputs Measure keeps to summary counts; the racadv models' folders, whose outputs Measure keeps to summary counts.

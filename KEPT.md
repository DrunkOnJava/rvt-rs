# Measure run 42556745265: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/42556745265, whose head side measured `8ddb0ad226919d2263572fe1abab75cf72f273ee`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 501 files for 32 models
- `base/`, from `measure-base`: 500 files for 32 models
- `compare/`, from `compare-scorer-diff`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output; the 2 sections of `scorer-diff.txt` that diff a left-out folder; the holdout models' folders, whose outputs Measure keeps to summary counts.

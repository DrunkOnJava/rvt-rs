# Measure run 46013208552: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/46013208552, whose head side measured `1cf8425503975f493dcbf7851e613c53fb184463`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 2559 files for 102 models
- `base/`, from `measure-base`: 2557 files for 102 models
- `verdict/`, from `measure-verdict`: 1 file
- `compare/`, from `compare-scorer-diff`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output; the 1 section of `scorer-diff.txt` that diff a left-out folder; the 16 lines of `verdict.md` with a left-out folder in a path; the holdout models' folders, whose outputs Measure keeps to summary counts; the name of 1 left-out model in `verdict.md`, written as `a left-out model`; the racadv models' folders, whose outputs Measure keeps to summary counts.

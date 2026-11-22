# Measure run 43525326226: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/43525326226, whose head side measured `334c3a1b4b78c01f39b505e5a82a0606f7952818`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 2616 files for 121 models
- `base/`, from `measure-base`: 2614 files for 121 models
- `verdict/`, from `measure-verdict`: 1 file
- `compare/`, from `compare-scorer-diff`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output; the bimcollab models' folders, whose outputs Measure keeps to summary counts; the holdout models' folders, whose outputs Measure keeps to summary counts; the racadv models' folders, whose outputs Measure keeps to summary counts.

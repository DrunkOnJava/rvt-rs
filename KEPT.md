# Measure run 42035308270: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/42035308270, whose head side measured `ea0c74499bdc4346435cf89c8fd95856371127cb`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 2015 files for 93 models
- `base/`, from `measure-base`: 2014 files for 93 models
- `compare/`, from `compare-scorer-diff`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output; the holdout models' folders, whose outputs Measure keeps to summary counts.

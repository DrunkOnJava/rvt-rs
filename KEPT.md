# Measure run 45074892701: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/45074892701, whose head side measured `b452f96708aa3564b5376c550b3bd32e198513f5`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 13 files for 4 models
- `base/`, from `measure-base`: 13 files for 4 models
- `verdict/`, from `measure-verdict`: 1 file
- `compare/`, from `compare-scorer-diff`: 1 file
- `pairs/`, from `measure-pairs`: 4 files

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output.

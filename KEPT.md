# Measure run 42106731054: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/42106731054, whose head side measured `c1a2a7399d4ee71bb9a10e3cdc4e6927884ca45c`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 11 files for 5 models
- `base/`, from `measure-base`: 11 files for 5 models
- `compare/`, from `compare-scorer-diff`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output.

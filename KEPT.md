# Measure run 45074900834: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/45074900834, whose head side measured `c1a2a7399d4ee71bb9a10e3cdc4e6927884ca45c`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 10 files for 3 models
- `base/`, from `measure-base`: 10 files for 3 models
- `verdict/`, from `measure-verdict`: 1 file
- `compare/`, from `compare-scorer-diff`: 1 file
- `pairs/`, from `measure-pairs`: 3 files

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output.

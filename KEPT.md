# Measure run 41850769925: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/41850769925, whose head side measured `e024f03c9cd14837c8ed49d0ab38d1fd414ff02b`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 201 files for 34 models

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output.

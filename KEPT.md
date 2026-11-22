# Measure run 42108271073: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/42108271073, whose head side measured `41528e94ceb7f33c0b12d052a5fe7b71325da905`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 107 files for 21 models

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output.

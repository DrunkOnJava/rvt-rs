# Measure run 41805333248: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/41805333248, whose head side measured `aaa38de808f1731acf6f6a6dace59904f873335b`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 354 files for 10 models

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output.

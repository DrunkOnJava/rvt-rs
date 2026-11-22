# Measure run 42612357300: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/42612357300, whose head side measured `b969296e46fd86e712b92a6c93cc1791b2d3cc1b`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 218 files for 31 models

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output.

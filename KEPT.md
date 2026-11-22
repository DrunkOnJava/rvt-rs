# Measure run 40932136360: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/40932136360, whose head side measured `af500685905ed47c3629871ca126cbb70c1b95ca`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 26 files for 4 models

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output.

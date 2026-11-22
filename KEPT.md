# Measure run 42127619521: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/42127619521, whose head side measured `6b4058d17c07947543db4b6d2bc4c2e671fcedca`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 46 files for 9 models

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output.

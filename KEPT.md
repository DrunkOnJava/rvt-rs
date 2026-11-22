# Measure run 41636859789: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/41636859789, whose head side measured `42e07b8ff34d33df9183c06a4014af19dc78356b`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 93 files for 24 models

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output.

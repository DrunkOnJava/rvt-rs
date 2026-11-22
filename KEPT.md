# Measure run 45073138078: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/45073138078, whose head side measured `ef98f44407a79b290b7ff933955771a535f6c1d1`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 13 files for 2 models
- `verdict/`, from `measure-verdict`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output.

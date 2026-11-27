# Measure run 46053386384: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/46053386384, whose head side measured `fb120a8381283b48eec5c8589249f41c98681e20`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 18 files for 3 models
- `verdict/`, from `measure-verdict`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output.

# Measure run 41787230413: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/41787230413, whose head side measured `16f6ac40ab699871a580366cffc67de53b776456`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 59 files for 16 models

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output.

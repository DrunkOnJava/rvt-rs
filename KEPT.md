# Measure run 41784415396: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/41784415396, whose head side measured `4532731d535663010438c2b5c00ace97dafc38a3`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 25 files for 9 models

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output.

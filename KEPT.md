# Measure run 42596736395: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/42596736395, whose head side measured `df44583fd1fcc76bbec7872b763d3fc5a06a612e`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 2102 files for 92 models

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output.

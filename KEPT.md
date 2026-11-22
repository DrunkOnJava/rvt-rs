# Measure run 41713761592: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/41713761592, whose head side measured `8fc583a165267ef842138d299472e7daecdd2bfe`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 2 files for 0 models

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output; the holdout models' folders, whose outputs Measure keeps to summary counts.

# Measure run 41985023986: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/41985023986, whose head side measured `b510ff3cd3c4971c907279bb91bef9cc9a197ccf`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 1928 files for 106 models
- `base/`, from `measure-base`: 1928 files for 106 models
- `compare/`, from `compare-scorer-diff`: 1 file

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output; the 4 sections of `scorer-diff.txt` that diff a left-out folder; the holdout models' folders, whose outputs Measure keeps to summary counts.

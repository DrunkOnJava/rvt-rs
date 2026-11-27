# Measure run 46058172915: kept outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/46058172915, whose head side measured `18d37a148473f7d897af0f53179e83624f5ca293`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 2613 files for 120 models
- `base/`, from `measure-base`: 2611 files for 120 models
- `verdict/`, from `measure-verdict`: 1 file
- `compare/`, from `compare-scorer-diff`: 1 file
- `timing/`, from `measure-timing`: 3 files

Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps and the probes' own output; the holdout models' folders, whose outputs Measure keeps to summary counts; the racadv models' folders, whose outputs Measure keeps to summary counts.

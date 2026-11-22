# Measure run 42106436497: kept probe outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/42106436497, whose head side measured `b2b5716ed3e05c7b5050a6351c7da18135330600`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 59 files for 58 models

Left out: everything but the probes' own output and `commit.txt`, which `refs/evidence/42106436497` keeps.

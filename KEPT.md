# Measure run 42108271073: kept probe outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/42108271073, whose head side measured `41528e94ceb7f33c0b12d052a5fe7b71325da905`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 22 files for 21 models

Left out: everything but the probes' own output and `commit.txt`, which `refs/evidence/42108271073` keeps.

# Measure run 45997957951: kept probe outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/45997957951, whose head side measured `739406904c3a5cfc122b4e9d2d95fa5882c32c1a`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 102 files for 101 models

Left out: everything but the probes' own output and `commit.txt`, which `refs/evidence/45997957951` keeps.

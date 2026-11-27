# Measure run 46029833703: kept probe outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/46029833703, whose head side measured `3aed43e6f933a194732ec0a8492c584bd161d78c`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 84 files for 83 models

Left out: everything but the probes' own output and `commit.txt`, which `refs/evidence/46029833703` keeps.

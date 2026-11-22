# Measure run 41787230413: kept probe outputs

From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/41787230413, whose head side measured `16f6ac40ab699871a580366cffc67de53b776456`, by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:

- `head/`, from `measure-head`: 33 files for 16 models

Left out: everything but the probes' own output and `commit.txt`, which `refs/evidence/41787230413` keeps.

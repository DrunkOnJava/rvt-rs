# Run 42125048710: kept log lines

From the log of https://github.com/DrunkOnJava/rvt-rs/actions/runs/42125048710, by `tools/ci/keep-run-outputs.py` (#2124, #2148), which says what is kept and why: the lines in which the pattern `^(Saved parameters|saved parameters|Read cost|A read under a) |^dist/assets/index-[A-Za-z0-9_-]+\.js ` is found.

- `log/lines.txt`: 8 lines from `browser tests 1/3`, `browser tests 3/3`, `browser tests 2/3`

Left out: every other line of the log.

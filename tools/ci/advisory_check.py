#!/usr/bin/env python3
"""Run one advisory check, and tell a database that never came from an advisory (#1457).

    advisory_check.py NAME COMMAND [ARGS...]

Runs COMMAND (`cargo audit ...` or `cargo deny ... check advisories`). If it passes, exit 0. If it
fails and its output says the advisory database (or the crate index it needs) could not be
fetched, it is run again, up to TRIES times with a pause that grows by PAUSE seconds each time;
if the database never comes, the check was NOT MEASURED: a `::warning::` says so, in the form
tools/ci/not_measured.py follows from one run to the next, and the exit is 0, because a host
that is down is not an advisory. Any other failure, an advisory among them, exits with COMMAND's
status at once and is never retried.
"""
import os
import re
import subprocess
import sys
import time

TRIES = 3
PAUSE = int(os.environ.get("ADVISORY_PAUSE_SECONDS", "30"))
SUBJECT = "RustSec advisory database"
# What cargo-audit 0.22 and cargo-deny 0.20 print when a fetch fails (and cargo, for the index).
FETCH_FAILED = re.compile(
    r"couldn't fetch advisory database|failed to fetch advisory database|spurious network error"
    r"|unable to update registry|could not resolve host|failed to download",
    re.IGNORECASE,
)


# A finding outranks a fetch message: an advisory is never read as a host being down.
FINDING = re.compile(
    r"RUSTSEC-\d{4}-\d{4}|vulnerabilit(?:y|ies) found|error\[(?:vulnerability|yanked|unsound|unmaintained|notice)\]",
    re.IGNORECASE,
)


def classify(status, output):
    """'pass', 'fetch' (the database or index could not be fetched) or 'fail'."""
    if status == 0:
        return "pass"
    if FINDING.search(output):
        return "fail"
    return "fetch" if FETCH_FAILED.search(output) else "fail"


def main(argv):
    if len(argv) < 3:
        print(__doc__)
        return 2
    name, command = argv[1], argv[2:]
    for attempt in range(1, TRIES + 1):
        done = subprocess.run(command, capture_output=True, text=True)
        output = done.stdout + done.stderr
        print(output, end="")
        verdict = classify(done.returncode, output)
        if verdict == "pass":
            return 0
        if verdict == "fail":
            return done.returncode or 1
        print(f"::notice::{name}: the advisory database could not be fetched (try {attempt} of {TRIES})")
        if attempt < TRIES:
            time.sleep(PAUSE * attempt)
    print(f"::warning::{SUBJECT} NOT MEASURED: {name} could not fetch it in {TRIES} tries, so nothing was audited")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

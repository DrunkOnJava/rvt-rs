"""Fail when an evidence page cites a workflow run that research/kept-runs.tsv does not list (#2124).

The README and the evidence pages (docs/supported-profile.md, docs/reference-models.md,
docs/evidence.md, docs/status.md) cite workflow runs by id, and GitHub deletes a run's artifacts
90 days after it ran. research/kept-runs.tsv lists every cited run: its outputs kept under
refs/evidence/<run> by .github/workflows/keep-run-outputs.yml, or `-` and why where nothing was
left to keep. A row named `<run>-probes` or `<run>-holdout` is a commit kept beside the run's
first; the run itself still needs its own row. A page that cites a new run without a row lets
that run's outputs expire unkept, so this fails until the workflow has kept it and its row is in.
"""

import pathlib
import re
import sys

root = pathlib.Path(__file__).resolve().parents[2]
PAGES = (
    "README.md",
    "docs/supported-profile.md",
    "docs/reference-models.md",
    "docs/evidence.md",
    "docs/status.md",
)
RUN = re.compile(r"github\.com/DrunkOnJava/rvt-rs/actions/runs/(\d+)")


def main():
    listed = set()
    for line in (root / "research/kept-runs.tsv").read_text(encoding="utf-8").splitlines():
        if line and not line.startswith("#") and not line.startswith("run\t"):
            name = line.split("\t", 1)[0]
            if re.fullmatch(r"\d+", name):
                listed.add(name)
    missing = []
    for page in PAGES:
        text = (root / page).read_text(encoding="utf-8")
        for number, line in enumerate(text.splitlines(), 1):
            for run in RUN.findall(line):
                if run not in listed:
                    missing.append((page, number, run))
    for page, number, run in missing:
        print(
            f"::error file={page},line={number}::run {run} is cited but research/kept-runs.tsv does not list it: "
            "keep it with the Keep run outputs workflow and add its row"
        )
    cited = {run for page in PAGES for run in RUN.findall((root / page).read_text(encoding="utf-8"))}
    print(f"Cited runs: {len(cited)}, every one in research/kept-runs.tsv" if not missing else f"{len(missing)} citations of runs the index lacks")
    return 1 if missing else 0


if __name__ == "__main__":
    sys.exit(main())

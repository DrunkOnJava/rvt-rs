"""Fail when a report or doc cites a commit GitHub does not have.

A report's reproduce commands name the commits its Measure runs built
(`-f ref=<sha>`, `-f base=<sha>`), never a branch, since branches are
deleted when they land. A hand-copied hash that was never pushed makes the
command fail for everyone who runs it: RE-266 and RE-296 each cited one
(2026-10-31). Every such hash in `reports/` and `docs/` is looked up in the
repository through the GitHub API.

A cited commit that exists but no branch holds can be collected by GitHub;
it is kept under `refs/archive/<date>/cited/` (docs/maintaining.md).

Needs network access. `GH_TOKEN` or `GITHUB_TOKEN` raises the API's rate
limit. A lookup that still fails after three tries is reported NOT MEASURED
and does not fail the check; only a commit GitHub says it does not have
does.
"""

import json
import os
import pathlib
import re
import sys
import time
import urllib.error
import urllib.request

root = pathlib.Path(__file__).resolve().parents[2]
repo = os.environ.get("GITHUB_REPOSITORY", "DrunkOnJava/rvt-rs")
token = os.environ.get("GH_TOKEN") or os.environ.get("GITHUB_TOKEN")
cited = re.compile(r"\b(?:ref|base)=([0-9a-f]{7,40})\b")


def lookup(sha):
    """True when GitHub has the commit, False when it says it has not,
    None when it could not be asked."""
    request = urllib.request.Request(f"https://api.github.com/repos/{repo}/commits/{sha}")
    request.add_header("Accept", "application/vnd.github+json")
    if token:
        request.add_header("Authorization", f"Bearer {token}")
    for attempt in range(3):
        try:
            with urllib.request.urlopen(request, timeout=20) as response:
                return json.load(response).get("sha", "").startswith(sha)
        except urllib.error.HTTPError as error:
            if error.code in (404, 422):
                return False
        except (urllib.error.URLError, TimeoutError, json.JSONDecodeError):
            pass
        time.sleep(2**attempt)
    return None


places = {}
for path in sorted([*root.joinpath("reports").rglob("*.md"), *root.joinpath("docs").rglob("*.md")]):
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        for sha in cited.findall(line):
            places.setdefault(sha, []).append(f"{path.relative_to(root)}:{number}")

missing, unmeasured = [], []
for sha, where in sorted(places.items()):
    found = lookup(sha)
    if found is False:
        missing.append((sha, where))
    elif found is None:
        unmeasured.append(sha)

for sha, where in missing:
    for place in where:
        print(f"{place}: cites commit {sha}, which {repo} does not have")
if unmeasured:
    print(f"::warning::cited commits NOT MEASURED: {len(unmeasured)} of {len(places)} could not be looked up")
print(f"{len(places)} cited commits, {len(missing)} missing, {len(unmeasured)} not measured")
sys.exit(1 if missing else 0)

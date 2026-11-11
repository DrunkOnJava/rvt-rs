"""Fail a pull request whose new report takes an RE id that is already taken.

An `RE-NN` id names one finding and one report, `reports/**/RE-NN-*.md`
(reports/README.md: ids are assigned in order and never reused;
docs/maintaining.md: when two open pull requests claim a number, the later
one is renumbered). Two reports under one id make every citation of it
ambiguous, and renumbering after both have landed means rewriting the code,
the probes and the changelog entries that cite it. On 2026-11-10 two pull
requests each carried an RE-392 before either was opened.

For a pull request this reads the reports it ADDS (the API's file list,
status `added`; a report it only edits or renames is not new) and fails when
one of them

- has an id that another report of the checked-out tree already has: the tree
  of a pull request's checkout is the merge of the pull request into `main`,
  so another path with the id is `main`'s, or the pull request's own second
  file;
- has an id that an EARLIER open pull request (a lower number) adds too, at
  another path. The later one fails, as docs/maintaining.md says; the earlier
  one is not blamed for it. The SAME path is no clash: a pull request stacked
  on the earlier one carries its commits, so its file list holds the report
  the earlier one added, and that is one report seen twice, not two reports
  under one id (a stacked pull request used to be told to renumber a report
  that was not its own).

Needs `GH_TOKEN` or `GITHUB_TOKEN` and the pull request's event. Anything that
keeps a lookup from being made (no token, the network, a rate limit, a server
error) is reported NOT MEASURED and does not fail the check; only a reused id
does. A push to a branch checks nothing.
"""

import json
import os
import re
import subprocess
import sys
import time
import urllib.error
import urllib.request

# A report is a markdown file under reports/ whose name starts with its id.
REPORT = re.compile(r"^reports/(?:.+/)?(RE-\d+)(?=[-.])[^/]*\.md$")
MAX_OPEN_PULL_REQUESTS = 100
MAX_FILE_PAGES = 3


def report_id(path):
    found = REPORT.match(path)
    return found.group(1) if found else None


def reused(added, tree, earlier):
    """The problems of a pull request that adds the reports `added` (paths).

    `tree` is every path of the checked-out tree and `earlier` maps the number
    of each earlier open pull request to the paths it adds. Pure, so its
    failure modes can be tried without a network."""
    problems = []
    mine = {}
    for path in sorted(added):
        found = report_id(path)
        if found:
            mine.setdefault(found, []).append(path)
    on_main = {}
    for path in tree:
        found = report_id(path)
        if found:
            on_main.setdefault(found, []).append(path)
    for found, paths in sorted(mine.items()):
        if len(paths) > 1:
            problems.append(f"{found} names {len(paths)} new reports: {', '.join(paths)}")
        others = [p for p in on_main.get(found, []) if p not in paths]
        if others:
            problems.append(f"{found} is already the id of {', '.join(others)} on main: take the next free id")
        for number, theirs in sorted(earlier.items()):
            clash = [p for p in theirs if report_id(p) == found and p not in paths]
            if clash:
                problems.append(f"{found} is also added by #{number}, opened first ({', '.join(clash)}): renumber this report")
    return problems


def api(path, token):
    """The decoded JSON of a GitHub API GET, or None when it could not be made."""
    request = urllib.request.Request(
        f"https://api.github.com/{path}",
        headers={
            "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
            **({"Authorization": f"Bearer {token}"} if token else {}),
        },
    )
    for attempt in range(3):
        try:
            with urllib.request.urlopen(request, timeout=20) as response:
                return json.load(response)
        except urllib.error.HTTPError as error:
            if error.code in (401, 403, 404, 429):
                return None
        except (OSError, ValueError):
            pass
        time.sleep(2 * (attempt + 1))
    return None


def added_reports(repo, number, token):
    """The report paths a pull request adds, or None when its files could not be listed."""
    out = []
    for page in range(1, MAX_FILE_PAGES + 1):
        files = api(f"repos/{repo}/pulls/{number}/files?per_page=100&page={page}", token)
        if files is None:
            return None
        out += [f["filename"] for f in files if f.get("status") == "added" and report_id(f["filename"])]
        if len(files) < 100:
            break
    return out


def main():
    if os.environ.get("GITHUB_EVENT_NAME") != "pull_request":
        print("RE ids: not a pull request, nothing to check.")
        return 0
    event = json.load(open(os.environ["GITHUB_EVENT_PATH"]))
    number = event["pull_request"]["number"]
    repo = os.environ.get("GITHUB_REPOSITORY", "DrunkOnJava/rvt-rs")
    token = os.environ.get("GH_TOKEN") or os.environ.get("GITHUB_TOKEN")
    mine = added_reports(repo, number, token)
    if mine is None:
        print(f"::warning::RE ids NOT MEASURED: the files of #{number} could not be listed")
        return 0
    if not mine:
        print(f"RE ids: #{number} adds no report.")
        return 0
    tree = subprocess.run(["git", "ls-files", "--", "reports"], capture_output=True, text=True, check=True).stdout.split("\n")
    earlier = {}
    open_pulls = api(f"repos/{repo}/pulls?state=open&per_page={MAX_OPEN_PULL_REQUESTS}", token)
    if open_pulls is None:
        print("::warning::RE ids NOT MEASURED against the other open pull requests: the list could not be read")
    else:
        for pull in open_pulls:
            if pull["number"] >= number:
                continue
            theirs = added_reports(repo, pull["number"], token)
            if theirs is None:
                print(f"::warning::RE ids NOT MEASURED against #{pull['number']}: its files could not be listed")
            elif theirs:
                earlier[pull["number"]] = theirs
    problems = reused(mine, tree, earlier)
    for problem in problems:
        print(f"::error::{problem}")
    if problems:
        return 1
    print(f"RE ids: the {len(mine)} new report(s) of #{number} take ids that main and the earlier open pull requests do not have.")
    return 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Open one issue for a failed scheduled check, unless one under its label is already open (#1457).

    open_issue.py open --label scheduled:x --title T --body B [--label type:ci] [--dry-run]
    open_issue.py failed-jobs [--exclude NAME]... [--dry-run]

An issue already open under the first label is left alone: it gets no comment, no edit and no
label. A check that keeps failing therefore leaves one open issue, and the next failure after
it is closed opens a new one. A dry run reads what is open and prints what would be opened.

`failed-jobs` opens one issue for each job of the current run (GITHUB_RUN_ID) that failed or
was cancelled, labelled `scheduled:` and the job's name in lower case. A job that exceeds its
`timeout-minutes` ends as cancelled, not as failed, so a cancelled job in a run nobody
cancelled is a timeout. The calling workflow runs this only `if: !cancelled()`, which is false
when the run itself was cancelled (by a person or a newer run), so such a run reports nothing.
The reporting job itself is still running when it looks, so it is never in the list.

Needs the GitHub CLI with GH_TOKEN (or a login), GH_REPO or GITHUB_REPOSITORY, and `issues: write`
(`actions: read` for failed-jobs).
"""
import argparse
import json
import os
import re
import subprocess
import sys
import time

LABEL_COLOUR = "1d76db"
# "cancelled" is how a job that exceeded timeout-minutes ends (see the module docstring).
FAILED = ("failure", "timed_out", "cancelled")
TIMED_OUT = ("cancelled", "timed_out")
LABEL_LIMIT = 50
PREFIX = "scheduled:"


def slug(text, limit=LABEL_LIMIT - len(PREFIX)):
    """Lower case letters and digits joined by single hyphens, at most `limit` characters."""
    words = re.sub(r"[^a-z0-9]+", "-", text.lower()).strip("-")
    return words[:limit].strip("-") or "check"


def reportable(jobs, exclude=()):
    """The jobs that failed, timed out or were cancelled, but for those named in `exclude`."""
    return [j for j in jobs if j.get("conclusion") in FAILED and j["name"] not in exclude]


def job_issue(job, workflow, repo, run_id, server="https://github.com"):
    """The issue to open for a failed job: its labels, title and body."""
    label = PREFIX + slug(job["name"])
    steps = ", ".join(job.get("steps") or []) or "none recorded"
    timed_out = job.get("conclusion") in TIMED_OUT
    how = "ended cancelled, which is how a job that exceeded its timeout-minutes ends (the run was not cancelled)" if timed_out else "failed"
    body = (
        f'The scheduled check "{job["name"]}" of the workflow "{workflow}" {how}.\n\n'
        f"- Run: {server}/{repo}/actions/runs/{run_id}\n"
        f"- Job: {job.get('html_url') or 'not recorded'}\n"
        f"- Step that failed or was running: {steps}\n\n"
        f"Opened by tools/ci/open_issue.py because no issue labelled `{label}` was open. While this "
        "issue stays open a later failure of the same check posts nothing here; close it once the "
        "check passes again."
    )
    title = f"Scheduled check {'did not finish' if timed_out else 'failed'}: {job['name']}"
    return {"labels": [label, "type:ci"], "title": title, "body": body}


class GhError(Exception):
    pass


def gh(*args, retries=2):
    """Run gh and return its output. A server error (GitHub answers 502 now and then) is retried
    `retries` times; a call that creates something passes retries=0."""
    for attempt in range(retries + 1):
        p = subprocess.run(["gh", *args], capture_output=True, text=True)
        if p.returncode == 0:
            return p.stdout
        message = (p.stderr or p.stdout).strip()
        if attempt < retries and re.search(r"HTTP 5\d\d|timed? ?out|connection", message, re.I):
            time.sleep(5 * (attempt + 1))
            continue
        raise GhError(f"gh {' '.join(args[:2])} failed: {message[:300]}")


def repo_name(arg):
    name = arg or os.environ.get("GH_REPO") or os.environ.get("GITHUB_REPOSITORY")
    if not name:
        raise SystemExit("no repository: pass --repo or set GH_REPO")
    return name


def open_issue(repo, labels, title, body, description="", dry_run=False):
    """Open the issue unless one is open under labels[0]; returns what happened."""
    existing = json.loads(gh("issue", "list", "--repo", repo, "--label", labels[0], "--state", "open",
                             "--json", "number,url", "--limit", "5"))
    if existing:
        first = existing[0]
        print(f"left alone: #{first['number']} {first['url']} is open under {labels[0]}")
        return "left alone"
    if dry_run:
        print(f"dry run, would open an issue under {', '.join(labels)}\n  title: {title}\n  body:")
        print("\n".join(f"    {line}" for line in body.splitlines()))
        return "would open"
    for label in labels:
        p = subprocess.run(["gh", "label", "create", label, "--repo", repo, "--color", LABEL_COLOUR,
                            "--description", description or "A scheduled check failed"], capture_output=True, text=True)
        if p.returncode != 0 and "already exists" not in (p.stderr + p.stdout):
            raise SystemExit(f"gh label create {label} failed: {(p.stderr or p.stdout).strip()[:300]}")
    args = ["issue", "create", "--repo", repo, "--title", title, "--body", body]
    for label in labels:
        args += ["--label", label]
    print(f"opened: {gh(*args, retries=0).strip()}")
    return "opened"


def failed_jobs(repo, run_id, exclude):
    out = gh("api", "--paginate", f"repos/{repo}/actions/runs/{run_id}/jobs?per_page=100&filter=latest", "--jq",
             '.jobs[] | {name, conclusion, html_url, steps: [.steps[] | select(.conclusion == "failure" or .conclusion == "cancelled") | .name]}')
    return reportable([json.loads(line) for line in out.splitlines() if line.strip()], exclude)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--repo")
    sub = ap.add_subparsers(dest="command", required=True)
    one = sub.add_parser("open")
    one.add_argument("--label", action="append", required=True, help="the first label is the one an open issue is looked up by")
    one.add_argument("--title", required=True)
    one.add_argument("--body", required=True)
    one.add_argument("--description", default="")
    one.add_argument("--dry-run", action="store_true")
    many = sub.add_parser("failed-jobs")
    many.add_argument("--exclude", action="append", default=[], help="a job name not to report")
    many.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()
    try:
        return run(args)
    except GhError as error:
        raise SystemExit(str(error))


def run(args):
    repo = repo_name(args.repo)
    if args.command == "open":
        open_issue(repo, args.label, args.title, args.body, args.description, args.dry_run)
        return 0
    run_id = os.environ.get("GITHUB_RUN_ID")
    if not run_id:
        raise SystemExit("failed-jobs needs GITHUB_RUN_ID")
    workflow = os.environ.get("GITHUB_WORKFLOW", "a workflow")
    server = os.environ.get("GITHUB_SERVER_URL", "https://github.com")
    jobs = failed_jobs(repo, run_id, set(args.exclude))
    print(f"{len(jobs)} failed job(s) in run {run_id}")
    for job in jobs:
        issue = job_issue(job, workflow, repo, run_id, server)
        open_issue(repo, issue["labels"], issue["title"], issue["body"], f"The scheduled check {job['name']} failed", args.dry_run)
    return 0


if __name__ == "__main__":
    sys.exit(main())

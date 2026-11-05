#!/usr/bin/env python3
"""Say when a check that passes with a NOT MEASURED warning has done so for too long (#1457).

    not_measured.py [--history] [--open] [--dry-run] [--max-runs 100]

Corpus health's BIM4LCA and R2025 fetches and the viewer dependency audit end with
`::warning::<subject> NOT MEASURED ...` when their host is down: the right call for one run,
since a host outage is not a defect of the commit. But a host that stays down, or a cache that is
gone while it does, leaves the check green with nothing measured. This reads the check-run
annotations of the newest `main` pushes of ci.yml and finds each subject whose warning is on
every one of the newest runs that measured it (a run in which the job failed or did not run
neither adds to a streak nor ends it; a run without the warning ends it), over at least MIN_RUNS
runs spanning at least MIN_HOURS. `--open` opens one issue for each, through open_issue.py: it
leaves an open one alone. Nothing here fails a run of CI on `main`.

Why 3 runs and 24 hours: ci.yml ran for 80 pushes to `main` in the 37.5 hours from 2026-11-04 03:33
to 2026-11-05 17:03 UTC, so three runs in a row can be minutes apart, and the outage of a host (the
npm advisory endpoint's maintenance windows) lasts hours. A subject warned on every run over a whole
day, three runs or more with none that measured it between, has been unmeasured through a night and
a morning, which no maintenance window explains.

Needs `gh` with GH_TOKEN, GH_REPO or GITHUB_REPOSITORY, and `actions: read` and `checks: read`
(`issues: write` for --open).
"""
import argparse
import json
import os
import re
import sys
from datetime import datetime

import open_issue

WORKFLOW = "ci.yml"
WATCHED = ("Corpus health (tier 2 — external)", "viewer dependency audit")
SUBJECT = re.compile(r"^(?P<subject>.+?) NOT MEASURED\b")
MIN_RUNS = 3
MIN_HOURS = 24.0


def subject_of(message):
    match = SUBJECT.match(message.strip())
    return match["subject"] if match else None


def streaks(runs, min_runs=MIN_RUNS, min_hours=MIN_HOURS):
    """What each subject's streak of warned runs is, newest first.

    `runs` is a list of {"id", "at" (a datetime), "jobs": {job name: set of subjects warned}}; a
    job name is a key only if the job succeeded in that run. Returns a list of dicts with the
    job, the subject, the number of runs in the streak, its hours, the newest and oldest run ids,
    `open` (True when the streak reaches the oldest run given, so an older one may continue it)
    and `persistent`.
    """
    runs = sorted(runs, key=lambda r: r["at"], reverse=True)
    found = []
    for job in sorted({j for r in runs for j in r["jobs"]}):
        considered = [r for r in runs if job in r["jobs"]]
        for subject in sorted(considered[0]["jobs"][job]):
            streak = []
            for r in considered:
                if subject not in r["jobs"][job]:
                    break
                streak.append(r)
            hours = (streak[0]["at"] - streak[-1]["at"]).total_seconds() / 3600
            found.append({
                "job": job, "subject": subject, "runs": len(streak), "hours": hours,
                "newest": streak[0]["id"], "oldest": streak[-1]["id"],
                "open": len(streak) == len(considered),
                "persistent": len(streak) >= min_runs and hours >= min_hours,
            })
    return found


def settled(runs, found):
    """True when every watched job has been seen and no streak can still change by looking at an older run."""
    if any(not any(job in r["jobs"] for r in runs) for job in WATCHED):
        return False
    return all(s["persistent"] or not s["open"] for s in found)


def gh_json(repo, path):
    return json.loads(open_issue.gh("api", f"repos/{repo}/{path}"))


def read_run(repo, run):
    """The run as `streaks` takes it: the watched jobs that succeeded, with their NOT MEASURED subjects."""
    seen = {}
    try:
        jobs = gh_json(repo, f"actions/runs/{run['id']}/jobs?per_page=100&filter=latest")["jobs"]
        for job in jobs:
            if job["name"] in WATCHED and job["conclusion"] == "success":
                notes = gh_json(repo, f"check-runs/{job['id']}/annotations?per_page=100")
                seen[job["name"]] = {s for a in notes if (s := subject_of(a.get("message", "")))}
    except open_issue.GhError as error:
        print(f"run {run['id']} not read, so it neither adds to a streak nor ends one: {error}", file=sys.stderr)
        seen = {}
    return {"id": run["id"], "at": datetime.fromisoformat(run["created_at"].replace("Z", "+00:00")), "jobs": seen,
            "url": run["html_url"]}


def collect(repo, max_runs, stop_when_settled):
    listing = gh_json(repo, f"actions/workflows/{WORKFLOW}/runs?branch=main&event=push&status=completed&per_page={min(max_runs, 100)}")
    runs = []
    for run in listing["workflow_runs"][:max_runs]:
        if run["conclusion"] not in ("success", "failure"):
            continue
        runs.append(read_run(repo, run))
        if stop_when_settled and settled(runs, streaks(runs)):
            break
    return runs


def issue_for(streak, repo, urls, server="https://github.com"):
    label = f"{open_issue.PREFIX}not-measured-{open_issue.slug(streak['subject'], 25)}"
    body = (
        f'The check "{streak["job"]}" of ci.yml has passed with a NOT MEASURED warning for {streak["subject"]} '
        f"on each of the newest {streak['runs']} runs of `main` that ran it, over {streak['hours']:.0f} hours.\n\n"
        f"- Newest run: {urls[streak['newest']]}\n- Oldest run in the streak: {urls[streak['oldest']]}\n\n"
        "That warning is the right result for one run when the host is down, so the job passes. A host that stays "
        "down, or a cache that is gone while it does, leaves the check green with nothing measured. Check the host "
        "and the Actions cache; the next run of `main` that measures it ends the streak, and this issue can be closed.\n\n"
        "Opened by tools/ci/not_measured.py, which leaves an open issue under this label alone."
    )
    return {"labels": [label, "type:ci"], "title": f"NOT MEASURED for {streak['hours']:.0f} hours: {streak['subject']}", "body": body}


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--repo")
    ap.add_argument("--max-runs", type=int, default=100, help="at most 100, the size of one page")
    ap.add_argument("--history", action="store_true", help="read every run up to --max-runs and print each one's warnings")
    ap.add_argument("--open", action="store_true", help="open an issue for each persistent subject")
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()
    try:
        return run(args)
    except open_issue.GhError as error:
        raise SystemExit(str(error))


def run(args):
    repo = open_issue.repo_name(args.repo)
    runs = collect(repo, args.max_runs, stop_when_settled=not args.history)
    if args.history:
        for r in sorted(runs, key=lambda r: r["at"], reverse=True):
            cells = "; ".join(f"{job}: {', '.join(sorted(s)) or 'measured'}" for job, s in sorted(r["jobs"].items())) or "no watched job succeeded"
            print(f"{r['at']:%Y-%m-%d %H:%M} run {r['id']}: {cells}")
    found = streaks(runs)
    urls = {r["id"]: r["url"] for r in runs}
    print(f"read {len(runs)} runs of {WORKFLOW} on main; {len(found)} subject(s) warned on the newest run")
    for s in found:
        state = "PERSISTENT" if s["persistent"] else ("open" if s["open"] else "ended")
        print(f"  {s['subject']} ({s['job']}): {s['runs']} runs, {s['hours']:.1f} hours, {state}")
    persistent = [s for s in found if s["persistent"]]
    if not persistent:
        print("no NOT MEASURED warning has persisted")
    if args.open:
        server = os.environ.get("GITHUB_SERVER_URL", "https://github.com")
        for s in persistent:
            issue = issue_for(s, repo, urls, server)
            open_issue.open_issue(repo, issue["labels"], issue["title"], issue["body"], f"{s['subject']} NOT MEASURED", args.dry_run)
    return 0


if __name__ == "__main__":
    sys.exit(main())

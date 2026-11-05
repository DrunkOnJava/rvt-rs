#!/usr/bin/env python3
"""Say when a check that passes with a NOT MEASURED warning has done so for too long (#1457).

    not_measured.py [--history] [--open] [--dry-run] [--max-runs 100]

Corpus health's BIM4LCA and R2025 fetches, the viewer dependency audit, the cited-commits lookup
in the PII guard, the viewer test that reads the BIM4LCA office and the scheduled advisory audits
end with `::warning::<subject> NOT MEASURED ...` when their host is down: the right call for one
run, since a host outage is not a defect of the commit. But a host that stays down, or a cache
that is gone while it does, leaves the check green with nothing measured. This reads the check-run
annotations of every job of the newest runs of the SOURCES (ci.yml on pushes to `main`, and
scheduled-checks.yml's own daily runs) and finds each subject whose warning is on every one of
the newest runs in which the job that wrote it succeeded (a run in which that job failed or did
not run neither adds to a streak nor ends it; a run without the warning ends it), over at least
MIN_RUNS runs spanning at least MIN_HOURS. No list of jobs is kept: a job that starts to write
such a warning is read from its next run. `--open` opens one issue for each, through
open_issue.py: it leaves an open one alone. Nothing here fails a run of CI on `main`.

Why 3 runs and 24 hours: ci.yml ran for 80 pushes to `main` in the 37.5 hours from 2026-11-04 03:33
to 2026-11-05 17:03 UTC, so three runs in a row can be minutes apart, and the outage of a host (the
npm advisory endpoint's maintenance windows) lasts hours. A subject warned on every run over a whole
day, three runs or more with none that measured it between, has been unmeasured through a night and
a morning, which no maintenance window explains. The daily scheduled runs reach it after three days.

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

# The runs read: the workflow file, and the event whose runs on `main` count.
SOURCES = (
    {"workflow": "ci.yml", "event": "push"},
    {"workflow": "scheduled-checks.yml", "event": "schedule"},
)
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


def settled(runs, found, min_runs=MIN_RUNS):
    """True when reading an older run cannot change the answer: at least `min_runs` runs are read
    (the newest run's warning-writing job may have failed, which hides its streak until an older
    run) and every streak is persistent or has ended."""
    return len(runs) >= min_runs and all(s["persistent"] or not s["open"] for s in found)


def gh_json(repo, path):
    return json.loads(open_issue.gh("api", f"repos/{repo}/{path}"))


def read_run(repo, run):
    """The run as `streaks` takes it: each job that succeeded, with its NOT MEASURED subjects.

    A job's annotations are read only where its check run counts some, so a run costs one listing
    of the commit's check runs and one request for each job that wrote an annotation."""
    seen = {}
    try:
        page = 1
        while True:
            listing = gh_json(repo, f"commits/{run['head_sha']}/check-runs?per_page=100&page={page}")
            for check in listing["check_runs"]:
                if f"/actions/runs/{run['id']}/" not in (check.get("html_url") or "") or check["conclusion"] != "success":
                    continue
                subjects = set()
                if (check.get("output") or {}).get("annotations_count"):
                    for note in gh_json(repo, f"check-runs/{check['id']}/annotations?per_page=100"):
                        subject = subject_of(note.get("message", ""))
                        if subject:
                            subjects.add(subject)
                seen[check["name"]] = subjects
            if len(listing["check_runs"]) < 100:
                break
            page += 1
    except open_issue.GhError as error:
        print(f"run {run['id']} not read, so it neither adds to a streak nor ends one: {error}", file=sys.stderr)
        seen = {}
    return {"id": run["id"], "at": datetime.fromisoformat(run["created_at"].replace("Z", "+00:00")), "jobs": seen,
            "url": run["html_url"]}


def collect(repo, source, max_runs, stop_when_settled):
    path = f"actions/workflows/{source['workflow']}/runs?branch=main&event={source['event']}&status=completed&per_page={min(max_runs, 100)}"
    runs = []
    try:
        listing = gh_json(repo, path)["workflow_runs"]
    except open_issue.GhError as error:
        if "404" in str(error):
            print(f"{source['workflow']} is not on main yet: no runs to read")
            return runs
        raise
    for run in listing[:max_runs]:
        if run["conclusion"] not in ("success", "failure"):
            continue
        runs.append(read_run(repo, run))
        if stop_when_settled and settled(runs, streaks(runs)):
            break
    return runs


def issue_for(streak, repo, urls, server="https://github.com"):
    label = f"{open_issue.PREFIX}not-measured-{open_issue.slug(streak['subject'], 25)}"
    body = (
        f'The check "{streak["job"]}" of {streak.get("workflow", "a workflow")} has passed with a NOT MEASURED warning for '
        f"{streak['subject']} on each of the newest {streak['runs']} runs that ran it, over {streak['hours']:.0f} hours.\n\n"
        f"- Newest run: {urls[streak['newest']]}\n- Oldest run in the streak: {urls[streak['oldest']]}\n\n"
        "That warning is the right result for one run when the host is down, so the job passes. A host that stays "
        "down, or a cache that is gone while it does, leaves the check green with nothing measured. Check the host "
        "and the Actions cache; the next run that measures it ends the streak, and this issue can be closed.\n\n"
        "Opened by tools/ci/not_measured.py, which leaves an open issue under this label alone."
    )
    return {"labels": [label, "type:ci"], "title": f"NOT MEASURED for {streak['hours']:.0f} hours: {streak['subject']}", "body": body}


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--repo")
    ap.add_argument("--max-runs", type=int, default=100, help="at most 100 a source, the size of one page")
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
    persistent, urls = [], {}
    for source in SOURCES:
        runs = collect(repo, source, args.max_runs, stop_when_settled=not args.history)
        if args.history:
            for r in sorted(runs, key=lambda r: r["at"], reverse=True):
                warned = "; ".join(f"{job}: {', '.join(sorted(s))}" for job, s in sorted(r["jobs"].items()) if s)
                print(f"{r['at']:%Y-%m-%d %H:%M} run {r['id']} ({len(r['jobs'])} jobs succeeded): {warned or 'every job measured'}")
        found = streaks(runs)
        urls.update({r["id"]: r["url"] for r in runs})
        print(f"{source['workflow']} on main ({source['event']}): read {len(runs)} runs; {len(found)} subject(s) warned on the newest")
        for s in found:
            s["workflow"] = source["workflow"]
            state = "PERSISTENT" if s["persistent"] else ("open" if s["open"] else "ended")
            print(f"  {s['subject']} ({s['job']}): {s['runs']} runs, {s['hours']:.1f} hours, {state}")
            if s["persistent"]:
                persistent.append(s)
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

#!/usr/bin/env python3
"""Delete the GitHub Actions caches nobody can use again:

- those of pull requests that are no longer open, and those of deleted branches;
- on `main`, the superseded generations of each rust-cache family: a new lockfile
  or toolchain saves a new cache under a new key and the old ones are never read
  again, apart from the newest few that a build may still restore from;
- on `main`, the superseded overlay base databases of each CodeQL language and
  version: every analysis of `main` saves one under its commit, and the next
  analysis on that version reads only the newest;
- on `main`, the superseded generations of Measure's score cache, which a run
  restores from the newest of its prefix and saves anew, and all but the newest
  sixteen of its binary caches, one per commit built, which only runs of that
  commit read;
- on `main`, the superseded generations of the RE1 models' cache, which is keyed by
  what pins them (`re1-<sha256 of research/re1-models.tsv>`, and the commit-keyed
  entries it replaced): a changed pin list saves a new key and nothing reads the old one.

The caches of live branches, of open pull requests, of tags, and `main`'s newest
generations are kept. The repository's cache limit is 10 GB and GitHub evicts the
least recently used cache past it, so stale generations fill the limit and leave
no room to see a real problem: this is the chore behind the audit's "the Actions
cache is under 90% of its 10 GB limit" line.

Dry run by default: it prints what it would delete. Pass `--delete` to delete. It refuses to go on
(exit 1, nothing deleted) when the open pull request list is empty or the branch list has no
`main`: GitHub answers a read with an empty body now and then, and every live cache would read as
dead. A 403 that is a permission stops the run at once; one that is a rate limit is waited out.

Needs the GitHub CLI (`gh`), logged in with the `repo` scope.

Usage:

    python3 tools/maintainer/clean_caches.py [--repo OWNER/REPO] [--keep 2] [--delete]
"""

import argparse
import json
import re
import subprocess
import sys
import time

# Swatinem/rust-cache keys: v0-rust-<job>-<os>-<arch>-<toolchain hash>-<lockfile hash>.
RUST_CACHE_KEY = re.compile(r"(v0-rust-.+)-[0-9a-f]{8}")
# CodeQL's overlay base databases:
# codeql-overlay-base-database-<n>-<hash>-<language>-<CodeQL version>-<commit>-<run id>-<attempt>.
# The family keeps the CodeQL version: a pull request's analysis takes the
# highest enabled CLI version among the cached bases, so each version keeps its
# newest ones.
CODEQL_OVERLAY_KEY = re.compile(r"(codeql-overlay-base-database-\d+-[0-9a-f]+-[a-z]+-[0-9.]+)-[0-9a-f]{40}-\d+-\d+")
# Measure's score cache (tools/ci/measure-cache.py):
# measure-scores-v<format>-<main or x-name>-<shard>.<side>-<measured sha>-<run id>-<attempt>.
# A run restores the newest of its shard's prefix, so only the newest generations are read.
SCORE_CACHE_KEY = re.compile(r"(measure-scores-v\d+-.+?)(?:-[0-9a-f]{40})?-\d+-\d+")
# Measure's binary cache: rvt-bin-v<format>-<measured commit>-<os>-<arch>-<toolchain>-<profile>, one
# per commit built. A commit's key is read only by runs of that commit, which are
# the last few: the newest BINARY_GENERATIONS are kept.
BINARY_CACHE_KEY = re.compile(r"(rvt-bin-v\d+)-[0-9a-f]{40}-.+")
BINARY_GENERATIONS = 16
# The RE1 models (.github/actions/fetch-re1): re1-<sha256 of the pin list>, 64 hex digits. The
# keys it replaced, re1-<commit> and re1-oracle-<commit>, 40, are one family with it, so that
# an entry nothing reads any more is the oldest of the family and goes with the first new key.
RE1_CACHE_KEY = re.compile(r"(re1)-(?:oracle-)?[0-9a-f]{40,64}")


def gh(*args):
    p = subprocess.run(["gh", "api", *args], capture_output=True, text=True)
    if p.returncode != 0:
        raise RuntimeError((p.stderr or p.stdout).strip().splitlines()[0][:160] if (p.stderr or p.stdout).strip() else "gh failed")
    return p.stdout


def pages(path, tries=1):
    """A paginated endpoint as one list, unwrapping the `actions_caches` envelope.

    GitHub answers a read with an empty body and a success status now and then, which reads as an
    empty list, and a server error now and then: a list that must not be empty passes `tries`."""
    for attempt in range(tries):
        try:
            out, decoder, i, text = [], json.JSONDecoder(), 0, gh("--paginate", path)
            while i < len(text):
                while i < len(text) and text[i] in " \r\n\t":
                    i += 1
                if i >= len(text):
                    break
                obj, i = decoder.raw_decode(text, i)
                out.extend(obj.get("actions_caches", []) if isinstance(obj, dict) else obj)
        except RuntimeError:
            if attempt == tries - 1:
                raise
            out = []
        if out or attempt == tries - 1:
            return out
        time.sleep(5)
    return []


def refusal(open_prs, live_branches):
    """Why nothing may be deleted, or None. A cache of a pull request or branch is kept only while
    the pull request is in the open list and the branch in the branch list, so an empty list
    would read every one as dead: the repository always has `main`, and a list without it is
    wrong."""
    if "main" not in live_branches:
        return "the branch list has no main"
    if not open_prs:
        return "the open pull request list is empty"
    return None


def is_rate_limited(message):
    """True for a rate limit (a 429, or a 403 that says so), False for a 403 that is a permission."""
    return bool(re.search(r"\b429\b|rate limit|abuse|secondary", message, re.I))


def unusable(caches, open_prs, live_branches, keep):
    """The caches to delete, each with the reason."""
    doomed = []
    generations = {}
    for c in caches:
        ref = c["ref"]
        m = re.fullmatch(r"refs/pull/(\d+)/(merge|head)", ref)
        if m:
            if int(m.group(1)) not in open_prs:
                doomed.append((c, f"pull request #{m.group(1)} is not open"))
        elif ref.startswith("refs/heads/"):
            name = ref[len("refs/heads/"):]
            if name != "main" and name not in live_branches:
                doomed.append((c, f"branch {name} was deleted"))
            elif name == "main":
                k = (
                    RUST_CACHE_KEY.fullmatch(c["key"])
                    or CODEQL_OVERLAY_KEY.fullmatch(c["key"])
                    or SCORE_CACHE_KEY.fullmatch(c["key"])
                    or BINARY_CACHE_KEY.fullmatch(c["key"])
                    or RE1_CACHE_KEY.fullmatch(c["key"])
                )
                if k:
                    generations.setdefault(k.group(1), []).append(c)
    for family, lst in generations.items():
        lst.sort(key=lambda c: c["last_accessed_at"], reverse=True)
        for c in lst[(BINARY_GENERATIONS if family.startswith("rvt-bin-") else keep):]:
            doomed.append((c, f"superseded generation of {family}"))
    return doomed


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--repo", default="DrunkOnJava/rvt-rs")
    ap.add_argument("--keep", type=int, default=2, help="newest generations of each rust-cache, CodeQL, score or RE1 family kept on main")
    ap.add_argument("--delete", action="store_true", help="delete them; without it, only print")
    args = ap.parse_args()
    repo = args.repo

    caches = pages(f"repos/{repo}/actions/caches?per_page=100", tries=3)
    open_prs = {p["number"] for p in pages(f"repos/{repo}/pulls?state=open&per_page=100", tries=3)}
    live_branches = {b["name"] for b in pages(f"repos/{repo}/branches?per_page=100", tries=3)}
    why_not = refusal(open_prs, live_branches)
    if why_not:
        print(f"refusing to go on: {why_not}, after three reads, so the caches of live pull requests and branches "
              "would read as dead; nothing is deleted", file=sys.stderr)
        return 1

    doomed = unusable(caches, open_prs, live_branches, args.keep)
    freed = sum(c["size_in_bytes"] for c, _ in doomed)
    total = sum(c["size_in_bytes"] for c in caches)
    print(f"{len(caches)} caches, {total / 1024**3:.1f} GB; {len(doomed)} unusable ({freed / 1024**3:.1f} GB), {(total - freed) / 1024**3:.1f} GB kept")
    reasons = {}
    for c, why in doomed:
        kind = "superseded generations on main" if why.startswith("superseded") else "closed pull requests and deleted branches"
        n, size = reasons.get(kind, (0, 0))
        reasons[kind] = (n + 1, size + c["size_in_bytes"])
    for kind, (n, size) in reasons.items():
        print(f"  {n:4} caches, {size / 1024**3:.2f} GB: {kind}")
    if not args.delete:
        print("dry run; pass --delete to delete")
        return 0

    deleted = gone = failed = 0
    backoff = 0.0
    for c, _ in doomed:
        for _ in range(6):
            try:
                gh("-X", "DELETE", f"repos/{repo}/actions/caches/{c['id']}")
                deleted += 1
                break
            except RuntimeError as error:
                if "404" in str(error):
                    gone += 1
                    break
                if is_rate_limited(str(error)):
                    backoff = min(60.0, max(2.0, backoff * 2))
                    time.sleep(backoff)
                    continue
                if "403" in str(error):
                    print(f"  cache {c['id']}: {error}: the token cannot delete caches (it needs `actions: write`); "
                          f"stopping after {deleted} deleted", file=sys.stderr)
                    return 1
                failed += 1
                print(f"  cache {c['id']}: {error}", file=sys.stderr)
                break
        else:
            failed += 1
        time.sleep(0.15)
    print(f"deleted {deleted}, already gone {gone}, failed {failed}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())

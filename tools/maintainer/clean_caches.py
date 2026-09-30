#!/usr/bin/env python3
"""Delete the GitHub Actions caches nobody can use again:

- those of pull requests that are no longer open, and those of deleted branches;
- on `main`, the superseded generations of each rust-cache family: a new lockfile
  or toolchain saves a new cache under a new key and the old ones are never read
  again, apart from the newest few that a build may still restore from.

The caches of live branches, of open pull requests, of tags, and `main`'s newest
generations are kept. The repository's cache limit is 10 GB and GitHub evicts the
least recently used cache past it, so stale generations fill the limit and leave
no room to see a real problem: this is the chore behind the audit's "the Actions
cache is under 90% of its 10 GB limit" line.

Dry run by default: it prints what it would delete. Pass `--delete` to delete.

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


def gh(*args):
    p = subprocess.run(["gh", "api", *args], capture_output=True, text=True)
    if p.returncode != 0:
        raise RuntimeError((p.stderr or p.stdout).strip().splitlines()[0][:160] if (p.stderr or p.stdout).strip() else "gh failed")
    return p.stdout


def pages(path):
    """A paginated endpoint as one list, unwrapping the `actions_caches` envelope."""
    out, decoder, i, text = [], json.JSONDecoder(), 0, gh("--paginate", path)
    while i < len(text):
        while i < len(text) and text[i] in " \r\n\t":
            i += 1
        if i >= len(text):
            break
        obj, i = decoder.raw_decode(text, i)
        out.extend(obj.get("actions_caches", []) if isinstance(obj, dict) else obj)
    return out


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
                k = RUST_CACHE_KEY.fullmatch(c["key"])
                if k:
                    generations.setdefault(k.group(1), []).append(c)
    for family, lst in generations.items():
        lst.sort(key=lambda c: c["last_accessed_at"], reverse=True)
        for c in lst[keep:]:
            doomed.append((c, f"superseded generation of {family}"))
    return doomed


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--repo", default="DrunkOnJava/rvt-rs")
    ap.add_argument("--keep", type=int, default=2, help="newest generations of each rust-cache family kept on main")
    ap.add_argument("--delete", action="store_true", help="delete them; without it, only print")
    args = ap.parse_args()
    repo = args.repo

    caches = pages(f"repos/{repo}/actions/caches?per_page=100")
    open_prs = {p["number"] for p in pages(f"repos/{repo}/pulls?state=open&per_page=100")}
    live_branches = {b["name"] for b in pages(f"repos/{repo}/branches?per_page=100")}

    doomed = unusable(caches, open_prs, live_branches, args.keep)
    freed = sum(c["size_in_bytes"] for c, _ in doomed)
    total = sum(c["size_in_bytes"] for c in caches)
    print(f"{len(caches)} caches, {total / 1024**3:.1f} GB; {len(doomed)} unusable ({freed / 1024**3:.1f} GB), {(total - freed) / 1024**3:.1f} GB kept")
    reasons = {}
    for c, why in doomed:
        kind = "superseded rust-cache generations on main" if why.startswith("superseded") else "closed pull requests and deleted branches"
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
                if "429" in str(error) or "403" in str(error):
                    backoff = min(60.0, max(2.0, backoff * 2))
                    time.sleep(backoff)
                    continue
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

"""Fail when a report or doc cites a commit GitHub does not have.

A report's reproduce commands name the commits its Measure runs built
(`-f ref=<sha>`, `-f base=<sha>`), never a branch, since branches are
deleted when they land. A hand-copied hash that was never pushed makes the
command fail for everyone who runs it: RE-266 and RE-296 each cited one
(2026-10-31). Every such hash in `reports/` and `docs/` is looked up in the
repository, all in one GraphQL request, so the check costs one request of
the token's budget however many reports there are.

It proves only that GitHub has the commit. A cited commit that no branch,
archive ref or pull request head reaches can still be collected; it is kept
by hand under `refs/archive/<date>/cited/` (docs/maintaining.md).

Needs network access and `GH_TOKEN` or `GITHUB_TOKEN` (GraphQL takes no
anonymous requests). Anything that keeps a lookup from being made (no token,
the network, a rate limit, a server error, an ambiguous short hash) is
reported NOT MEASURED within two minutes and does not fail the check; only a
commit GitHub says it does not have does.
"""

import http.client
import json
import os
import pathlib
import re
import sys
import time
import urllib.error
import urllib.request

root = pathlib.Path(__file__).resolve().parents[2]
owner, name = os.environ.get("GITHUB_REPOSITORY", "DrunkOnJava/rvt-rs").split("/")
token = os.environ.get("GH_TOKEN") or os.environ.get("GITHUB_TOKEN")
deadline = time.monotonic() + 120
cited = re.compile(r"\b(?:ref|base)=([0-9a-f]{7,40})\b")
# A network or protocol failure, wherever in the request it happens.
unreachable = (OSError, http.client.HTTPException, json.JSONDecodeError, ValueError)


class Throttled(Exception):
    pass


def call(request):
    """The decoded JSON of `request`, or None when it could not be made;
    raises Throttled on a rate limit, after which nothing more is asked."""
    for attempt in range(3):
        if time.monotonic() > deadline:
            return None
        try:
            with urllib.request.urlopen(request, timeout=20) as response:
                return json.load(response)
        except urllib.error.HTTPError as error:
            if error.code in (403, 429):
                raise Throttled
            if error.code < 500:
                return {"status": error.code, "message": error.read().decode("utf-8", "replace")}
        except unreachable:
            pass
        if attempt < 2:
            time.sleep(2**attempt)
    return None


def request(url, body=None):
    request = urllib.request.Request(url, data=body)
    request.add_header("Accept", "application/vnd.github+json")
    request.add_header("Authorization", f"Bearer {token}")
    return request


def found_all(shas):
    """For each sha: True when GitHub has the commit, None when it does not
    say (absent: its answer is confirmed below)."""
    fields = " ".join(
        f'c{i}: object(expression: "{sha}") {{ ... on Commit {{ oid }} }}' for i, sha in enumerate(shas)
    )
    query = f'query {{ repository(owner: "{owner}", name: "{name}") {{ {fields} }} }}'
    answer = call(request("https://api.github.com/graphql", json.dumps({"query": query}).encode()))
    repository = ((answer or {}).get("data") or {}).get("repository")
    if repository is None:
        return None
    return {sha: (repository.get(f"c{i}") or {}).get("oid", "").startswith(sha) for i, sha in enumerate(shas)}


def missing_one(sha):
    """True when the REST API says the repository has no such commit, False
    when it has it, None when it cannot tell (an ambiguous short hash)."""
    answer = call(request(f"https://api.github.com/repos/{owner}/{name}/commits/{sha}"))
    if answer is None:
        return None
    if answer.get("status") in (404, 422):
        return None if "ambiguous" in answer.get("message", "").lower() else True
    return not answer.get("sha", "").startswith(sha)


places = {}
for path in sorted([*root.joinpath("reports").rglob("*.md"), *root.joinpath("docs").rglob("*.md")]):
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        for sha in cited.findall(line):
            # A hash of digits alone may be a number (`base=1048576`).
            if len(sha) >= 8 or re.search("[a-f]", sha):
                places.setdefault(sha, []).append(f"{path.relative_to(root)}:{number}")

shas = sorted(places)
missing, unmeasured = [], []
try:
    found = found_all(shas) if token else None
    if found is None:
        unmeasured = shas
    else:
        for sha in shas:
            if found[sha]:
                continue
            gone = missing_one(sha)
            if gone:
                missing.append(sha)
            elif gone is None:
                unmeasured.append(sha)
except Throttled:
    unmeasured = [sha for sha in shas if sha not in missing]

for sha in missing:
    for place in places[sha]:
        print(f"{place}: cites commit {sha}, which {owner}/{name} does not have")
if unmeasured:
    print(f"::warning::cited commits NOT MEASURED: {len(unmeasured)} of {len(shas)} could not be looked up")
print(f"{len(shas)} cited commits, {len(missing)} missing, {len(unmeasured)} not measured")
sys.exit(1 if missing else 0)

#!/usr/bin/env python3
"""What must hold before publish.yml builds or publishes anything (#1457).

A release is a signed tag on a commit whose CI passed, on main or on a hotfix
branch, whose version files all say the tag's version. Pushing the tag is the
release decision and nobody approves the run, so this refuses before the first
wheel builds. Everything that decides is a pure function of data already
fetched (tag_version, check_versions, tag_problems, branch_problems,
crate_state), so what must never publish is tested with no network, git or
clock (test_release_preflight.py); the I/O around them only fetches.

Usage:
  release_preflight.py check --event push|workflow_dispatch --ref-type tag|branch
                       --ref-name NAME --sha SHA [--repo OWNER/NAME] [--git-dir DIR]
      The tag, its signature, its commit's branch and the version files, read
      from SHA's tree. With workflow_dispatch there is no tag: the tag checks
      say so and pass, and the version files must still agree with each other.
      Prints every problem, writes version=X.Y.Z to $GITHUB_OUTPUT, exits 1 on
      any problem. Waiting for CI is the wait-for-check action's job.
  release_preflight.py crate-state CRATE VERSION
      "skip" when crates.io already has that version (a re-run after a partial
      release), "publish" when it does not; exits 1 when it cannot tell, or when
      the version is there but yanked. Writes state=... to $GITHUB_OUTPUT.
"""
import argparse
import json
import os
import re
import subprocess
import sys
import time
import tomllib
import urllib.error
import urllib.request

REPO = "DrunkOnJava/rvt-rs"

SEMVER = re.compile(
    r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)"
    r"(-((0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*)(\.(0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*))*))?",
    re.ASCII,
)

# path, package name, whether its own version is the release's, whether it pins rvt
MANIFESTS = (
    ("Cargo.toml", "rvt", True, False),
    ("rvt-py/Cargo.toml", "rvt-py", True, True),
    ("rvt-mcp/Cargo.toml", "rvt-rs-mcp", True, True),
    ("rvt-wasm/Cargo.toml", "rvt-wasm", True, False),
    ("tools/stream_evidence/Cargo.toml", "stream-evidence", False, True),
)
# the workspace crates each lock records at the release's version
LOCKS = {
    "Cargo.lock": ("rvt", "rvt-py", "rvt-rs-mcp", "rvt-wasm"),
    "fuzz/Cargo.lock": ("rvt",),
}
FILES = tuple(path for path, *_ in MANIFESTS) + tuple(LOCKS) + (
    "viewer/package.json", "CITATION.cff", "CHANGELOG.md", "pyproject.toml",
)

TRANSIENT_VERIFICATION = ("gpgverify_unavailable", "gpgverify_error", "ocsp_pending")
RELEASE_BRANCH = re.compile(r"origin/(main|hotfix/[^/].*)", re.ASCII)


def tag_version(ref):
    name = ref[len("refs/tags/"):] if ref.startswith("refs/tags/") else ref
    return name[1:] if name.startswith("v") and SEMVER.fullmatch(name[1:]) else None


def _table(value):
    return value if isinstance(value, dict) else {}


def _toml(files, path, problems):
    text = files.get(path)
    if not isinstance(text, str):
        problems.append(f"{path} could not be read")
        return None
    try:
        return tomllib.loads(text)
    except tomllib.TOMLDecodeError as error:
        problems.append(f"{path} is not valid TOML ({error})")
        return None


def _manifest(files, path, name, owns_version, pins_rvt, found, problems):
    data = _toml(files, path, problems)
    if data is None:
        return
    package = _table(data.get("package"))
    if package.get("name") != name:
        problems.append(f"{path} is the manifest of {package.get('name')!r}, not {name}")
        return
    if owns_version:
        version = package.get("version")
        if isinstance(version, str):
            found[f"{path} version"] = version
        else:
            problems.append(f"{path} has no literal package version")
    if pins_rvt:
        pin = _table(_table(data.get("dependencies")).get("rvt")).get("version")
        if not isinstance(pin, str):
            problems.append(f"{path} has no version pin on rvt")
        elif not pin.startswith("=") or not SEMVER.fullmatch(pin[1:]):
            problems.append(f"{path} pins rvt to {pin!r}, which is not an exact =X.Y.Z")
        else:
            found[f"{path} pin on rvt"] = pin[1:]


def _lock(files, path, names, found, problems):
    data = _toml(files, path, problems)
    if data is None:
        return
    packages = data.get("package")
    packages = packages if isinstance(packages, list) else []
    for name in names:
        ours = [_table(entry) for entry in packages if _table(entry).get("name") == name and "source" not in _table(entry)]
        if len(ours) != 1:
            problems.append(f"{path} has {len(ours)} workspace entries for {name}, not one")
        elif isinstance(ours[0].get("version"), str):
            found[f"{path} entry for {name}"] = ours[0]["version"]
        else:
            problems.append(f"{path} has no version for {name}")


def _package_json(files, found, problems):
    path = "viewer/package.json"
    text = files.get(path)
    if not isinstance(text, str):
        problems.append(f"{path} could not be read")
        return
    try:
        version = _table(json.loads(text)).get("version")
    except json.JSONDecodeError as error:
        problems.append(f"{path} is not valid JSON ({error})")
        return
    if isinstance(version, str):
        found[f"{path} version"] = version
    else:
        problems.append(f"{path} has no version")


def _citation(files, found, problems):
    path = "CITATION.cff"
    text = files.get(path)
    if not isinstance(text, str):
        problems.append(f"{path} could not be read")
        return
    versions = re.findall(r"^version:[ \t]*\"?([^\"\s#]+)\"?[ \t]*(?:#.*)?$", text, re.M)
    if len(versions) == 1:
        found[f"{path} version"] = versions[0]
    else:
        problems.append(f"{path} has {len(versions)} top-level version lines, not one")


def _pyproject(files, problems):
    path = "pyproject.toml"
    data = _toml(files, path, problems)
    if data is None:
        return
    project = _table(data.get("project"))
    maturin = _table(_table(data.get("tool")).get("maturin"))
    if "version" in project or "version" not in (project.get("dynamic") or []):
        problems.append(f"{path} must leave the wheel's version dynamic, so rvt-py/Cargo.toml is its one source")
    if maturin.get("manifest-path") != "rvt-py/Cargo.toml":
        problems.append(f"{path} builds the wheel from {maturin.get('manifest-path')!r}, not rvt-py/Cargo.toml")


def check_versions(files, expected):
    found = {}
    problems = []
    for path, name, owns_version, pins_rvt in MANIFESTS:
        _manifest(files, path, name, owns_version, pins_rvt, found, problems)
    for path, names in LOCKS.items():
        _lock(files, path, names, found, problems)
    _package_json(files, found, problems)
    _citation(files, found, problems)
    _pyproject(files, problems)
    distinct = sorted(set(found.values()))
    if expected is not None:
        version = expected
        for value in distinct:
            if value != expected:
                labels = [label for label, found_value in found.items() if found_value == value]
                problems.append(f"the tag says {expected}, but {value} is in {'; '.join(labels)}")
    elif len(distinct) == 1:
        version = distinct[0]
    else:
        version = None
        if distinct:
            where = ". ".join(f"{v} is in {'; '.join(l for l, x in found.items() if x == v)}" for v in distinct)
            problems.append(f"the version files disagree. {where}")
    changelog = files.get("CHANGELOG.md")
    if not isinstance(changelog, str):
        problems.append("CHANGELOG.md could not be read")
    elif version is not None and not re.search(r"^## \[" + re.escape(version) + r"\]", changelog, re.M):
        problems.append(f"CHANGELOG.md has no '## [{version}]' section")
    return version, problems


def _get(value, *keys):
    for key in keys:
        value = value.get(key) if isinstance(value, dict) else None
    return value


def verification_is_transient(tag_json):
    verification = _get(tag_json, "verification")
    return (
        isinstance(verification, dict)
        and verification.get("verified") is not True
        and verification.get("reason") in TRANSIENT_VERIFICATION
    )


def tag_problems(ref_json, tag_json, name, commit):
    if not isinstance(ref_json, dict) or not isinstance(_get(ref_json, "object"), dict):
        return [f"tag {name} was not found on GitHub"]
    kind, ref_sha = _get(ref_json, "object", "type"), _get(ref_json, "object", "sha")
    if kind != "tag":
        return [f"tag {name} is lightweight (it points straight at a {kind}); a release tag is annotated and signed: git tag -s"]
    if not isinstance(tag_json, dict):
        return [f"the tag object {ref_sha} of {name} could not be read"]
    problems = []
    if _get(tag_json, "sha") != ref_sha:
        problems.append(f"the tag object {_get(tag_json, 'sha')} is not the one {name} points at ({ref_sha})")
    if _get(tag_json, "tag") != name:
        problems.append(f"the tag object is for {_get(tag_json, 'tag')}, not {name}")
    target_kind, target = _get(tag_json, "object", "type"), _get(tag_json, "object", "sha")
    if target_kind != "commit":
        problems.append(f"the tag points at a {target_kind}, not a commit")
    elif target != commit:
        problems.append(f"the tag points at {str(target)[:12]} but this run is for {commit[:12]}")
    verification = _get(tag_json, "verification")
    if not isinstance(verification, dict):
        problems.append("GitHub gave no verification for the tag")
    elif verification.get("verified") is not True:
        problems.append(f"the tag's signature is not verified by GitHub (reason: {verification.get('reason')})")
    elif verification.get("reason") != "valid":
        problems.append(f"the tag's signature reason is {verification.get('reason')}, not valid")
    return problems


def branch_problems(containing):
    if "origin/main" not in containing:
        return ["origin/main is not in the checkout, so the fetch was incomplete (the job needs fetch-depth: 0)"]
    if any(RELEASE_BRANCH.fullmatch(ref) and contained is True for ref, contained in containing.items()):
        return []
    return ["the tagged commit is on neither origin/main nor an origin/hotfix/* branch"]


def crate_state(status, body, crate, version):
    if status == 404:
        return "publish", f"{crate} {version} is not on crates.io"
    if status != 200:
        return "error", f"crates.io answered HTTP {status} for {crate} {version}, so whether it is published is unknown"
    try:
        info = _get(json.loads(body), "version")
    except json.JSONDecodeError:
        return "error", f"crates.io's answer for {crate} {version} is not JSON"
    if not isinstance(info, dict) or info.get("num") != version:
        return "error", f"crates.io's answer for {crate} {version} is for {_get(info, 'num')!r}"
    if info.get("yanked") is not False:
        return "error", f"{crate} {version} is on crates.io but yanked, so this release cannot be completed under that version"
    return "skip", f"{crate} {version} is already on crates.io"


def http_get(url, token=None, tries=5):
    headers = {"User-Agent": f"rvt-rs-release-preflight (https://github.com/{REPO})", "Accept": "application/json"}
    if token and url.startswith("https://api.github.com/"):
        headers["Authorization"] = f"Bearer {token}"
        headers["X-GitHub-Api-Version"] = "2022-11-28"
    for attempt in range(1, tries + 1):
        try:
            with urllib.request.urlopen(urllib.request.Request(url, headers=headers), timeout=30) as reply:
                return reply.status, reply.read().decode()
        except urllib.error.HTTPError as error:
            status, body = error.code, error.read().decode(errors="replace")
            if status != 429 and status < 500:
                return status, body
        except (urllib.error.URLError, TimeoutError, OSError):
            status, body = 0, ""
        if attempt < tries:
            print(f"GET {url} gave {status or 'no answer'} (try {attempt}); retrying", file=sys.stderr)
            time.sleep(attempt * 5)
    return status, body


def git(git_dir, *args):
    return subprocess.run(["git", "-C", git_dir, *args], capture_output=True, text=True)


def read_blob(git_dir, sha, path):
    done = git(git_dir, "cat-file", "blob", f"{sha}:{path}")
    return done.stdout if done.returncode == 0 else None


def containing_branches(git_dir, sha):
    listed = git(git_dir, "for-each-ref", "--format=%(refname)", "refs/remotes/origin/main", "refs/remotes/origin/hotfix/")
    refs = [line for line in listed.stdout.split("\n") if line] if listed.returncode == 0 else []
    return {
        ref[len("refs/remotes/"):]: git(git_dir, "merge-base", "--is-ancestor", sha, ref).returncode == 0
        for ref in refs
    }


def fetch_tag(repo, name, token):
    base = f"https://api.github.com/repos/{repo}/git"
    status, body = http_get(f"{base}/ref/tags/{name}", token)
    if status == 404:
        return None, None
    if status != 200:
        raise OSError(f"GET git/ref/tags/{name} answered HTTP {status}")
    ref_json = json.loads(body)
    if _get(ref_json, "object", "type") != "tag":
        return ref_json, None
    tag_json = None
    for attempt in range(4):
        status, body = http_get(f"{base}/tags/{_get(ref_json, 'object', 'sha')}", token)
        if status != 200:
            raise OSError(f"GET git/tags/{_get(ref_json, 'object', 'sha')} answered HTTP {status}")
        tag_json = json.loads(body)
        if not verification_is_transient(tag_json):
            break
        print(f"GitHub could not verify the signature yet ({_get(tag_json, 'verification', 'reason')}); asking again", file=sys.stderr)
        time.sleep(10)
    return ref_json, tag_json


def write_output(key, value):
    path = os.environ.get("GITHUB_OUTPUT")
    if path:
        with open(path, "a") as out:
            out.write(f"{key}={value}\n")


def check(args):
    token = os.environ.get("GH_TOKEN") or os.environ.get("GITHUB_TOKEN")
    problems, notes = [], []
    expected = None
    if args.event == "push":
        if args.ref_type != "tag":
            problems.append(f"publish.yml releases tags; this run is for the {args.ref_type} {args.ref_name}")
        expected = tag_version(args.ref_name)
        if expected is None:
            problems.append(f"{args.ref_name!r} is not a release tag name (vMAJOR.MINOR.PATCH, with an optional -prerelease)")
        else:
            try:
                ref_json, tag_json = fetch_tag(args.repo, args.ref_name, token)
            except (OSError, json.JSONDecodeError) as error:
                problems.append(f"could not read the tag from GitHub ({error})")
            else:
                problems += tag_problems(ref_json, tag_json, args.ref_name, args.sha)
            problems += branch_problems(containing_branches(args.git_dir, args.sha))
    else:
        notes.append(f"{args.event} has no tag, so the tag name, signature, branch and CI checks do not apply")
    files = {path: read_blob(args.git_dir, args.sha, path) for path in FILES}
    version, version_problems = check_versions(files, expected)
    problems += version_problems
    for line in notes:
        print(f"preflight: {line}")
    for line in problems:
        print(f"::error::{line}" if os.environ.get("GITHUB_ACTIONS") else f"preflight: REFUSED: {line}")
    if problems:
        return 1
    write_output("version", version)
    print(f"preflight: ok, version {version}" + (f", tag {args.ref_name} on {args.sha[:12]}" if expected else ""))
    return 0


def crate(args):
    if not re.fullmatch(r"[a-z0-9_-]+", args.crate) or not SEMVER.fullmatch(args.version):
        print(f"::error::not a crate name and version: {args.crate!r} {args.version!r}")
        return 1
    status, body = http_get(f"https://crates.io/api/v1/crates/{args.crate}/{args.version}")
    state, message = crate_state(status, body, args.crate, args.version)
    if state == "error":
        print(f"::error::{message}")
        return 1
    print(f"{message}: {state}")
    write_output("state", state)
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest="command", required=True)
    one = commands.add_parser("check")
    one.add_argument("--event", required=True, choices=("push", "workflow_dispatch"))
    one.add_argument("--ref-type", required=True)
    one.add_argument("--ref-name", required=True)
    one.add_argument("--sha", required=True)
    one.add_argument("--repo", default=os.environ.get("GITHUB_REPOSITORY", REPO))
    one.add_argument("--git-dir", default=".")
    one.set_defaults(run=check)
    two = commands.add_parser("crate-state")
    two.add_argument("crate")
    two.add_argument("version")
    two.set_defaults(run=crate)
    args = parser.parse_args(argv)
    return args.run(args)


if __name__ == "__main__":
    sys.exit(main())

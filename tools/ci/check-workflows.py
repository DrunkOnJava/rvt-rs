#!/usr/bin/env python3
"""Rules about the workflows that no linter knows (CI's env config lint job; issue #1457).

    python3 tools/ci/check-workflows.py

Exits 1 with an `::error` line for each of:

* Timeouts. A job that runs steps sets `timeout-minutes` (the default is 360, six hours of a
  hung runner). A job that calls a workflow with `uses:` is left out: its timeouts live in
  the callee.
* Runner pins. No `runs-on`, or matrix that feeds it, names a `-latest` label: it moves to a
  new image without a commit here (ci.yml says why, at its `jobs:`).
* The MSRV. Both matrix strings of ci.yml's `test` job name Cargo.toml's `rust-version` for
  their `msrv` entry, and so does every other crate that declares one, but for the crates in
  DELIBERATE_MSRV.
* The documentation-only path list. ci.yml's `paths-ignore` (the commits that run no CI), the
  regex in ci.yml's `viewer-paths` job (the pull requests that skip the viewer tests) and the
  regex in Deploy viewer's wait step (the commits it deploys without waiting for CI) must
  name the same paths, but for the files in EXPECTED_BUILT_FROM.

PENDING lists the jobs another change is still editing. They are reported as warnings, so a
job added anywhere else without a timeout still fails.

The comparisons are pure functions, tested on the ways they can fail in
tools/ci/test_check_workflows.py. Reading the workflows needs PyYAML
(tools/ci/requirements-test.txt).
"""
import json
import re
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / ".github" / "workflows"

ALL = "*"

# Jobs that do not meet the rules yet, because the change that does it is in another pull
# request (issue #1457). A job here is a warning; delete its line with that change, which also
# fixes what the warning says. A job in these files that is not listed fails like any other.
# What each should get, from the longest successful run in the last 10 to 25 runs (2026-11-05):
#   ci.yml python-wheel: timeout-minutes 40 (windows leg longest 11.6), and macos-26 and
#     windows-2025-vs2026 in place of the -latest labels, as the test job has
#   codeql.yml analyze: 30 (rust leg longest 8.3)
#   deploy-viewer.yml build: 30 (longest 8.4)
#   publish.yml: build-wheels 40 (windows longest 8.8), build-sdist 10, validate-wheel-install 20,
#     release-source-smoke 45 (longest 12.2), docs-rs-preview 20, viewer-release-smoke 40
#     (longest 14.7), publish-crate 20, publish-testpypi 10, publish 10; macos-26 and
#     windows-2025-vs2026 for the -latest labels
#   release-binaries.yml: build 60 (windows leg longest 16.8), checksums 10, image 20, upload 15;
#     the same label pins
PENDING = {
    "ci.yml": {"python-wheel": "wheel lane"},
    "codeql.yml": {"analyze": "CodeQL lane"},
    "deploy-viewer.yml": {"build": "viewer WASM lane"},
    "publish.yml": {ALL: "release lane"},
    "release-binaries.yml": {ALL: "release lane"},
}

# Crates whose rust-version is not the workspace's, and the version each declares.
DELIBERATE_MSRV = {
    "rvt-mcp/Cargo.toml": "1.88",  # rmcp's minimum; nothing in `rvt` depends on this crate
}

# viewer-paths counts these two files as code, not documentation: the viewer is built from
# the demo catalogue and, through src/capability.rs, the support matrix. Deploy viewer's wait
# step and CI's paths-ignore treat them as documentation.
EXPECTED_BUILT_FROM = {"docs/viewer-demos.json", "docs/support-matrix.json"}

MISSING = object()
LATEST = re.compile(r"\b(?:ubuntu|windows|macos)-latest\b")


def pending_reason(pending, workflow, job):
    jobs = pending.get(workflow, {})
    return jobs.get(job) or jobs.get(ALL)


def bad_timeout(value):
    """Why `value` is not a usable timeout-minutes, or None."""
    if value is MISSING:
        return "runs steps and sets no timeout-minutes (the default is 360)"
    if isinstance(value, bool) or value is None:
        return f"has timeout-minutes {value!r}, not a number of minutes"
    if isinstance(value, int):
        if 1 <= value < 360:
            return None
        return f"has timeout-minutes {value}, which is not a limit below the default 360"
    if isinstance(value, str):
        if "${{" in value and "}}" in value:
            return None
        return f"has timeout-minutes {value!r}, neither a number nor an expression"
    return f"has timeout-minutes {value!r}, not a whole number of minutes"


def timeout_problems(workflows, pending):
    """(errors, notes) for jobs that run steps without a usable timeout."""
    errors, notes = [], []
    for name, doc in sorted(workflows.items()):
        for job_id, job in sorted(((doc or {}).get("jobs") or {}).items()):
            if "uses" in job or "steps" not in job:
                continue
            problem = bad_timeout(job.get("timeout-minutes", MISSING))
            if problem is None:
                continue
            why = pending_reason(pending, name, job_id)
            if why:
                notes.append(f"{name}: job '{job_id}' {problem} (pending: {why})")
            else:
                errors.append(f"{name}: job '{job_id}' {problem}")
    return errors, notes


def label_problems(workflows, pending):
    """(errors, notes) for jobs whose runs-on or matrix names a `-latest` runner."""
    errors, notes = [], []
    for name, doc in sorted(workflows.items()):
        for job_id, job in sorted(((doc or {}).get("jobs") or {}).items()):
            if "uses" in job:
                continue
            text = json.dumps([job.get("runs-on"), job.get("strategy")], default=str)
            found = sorted(set(LATEST.findall(text)))
            if not found:
                continue
            labels = ", ".join(sorted(set(re.findall(r"(?:ubuntu|windows|macos)-latest[A-Za-z0-9-]*", text))))
            problem = f"runs on {labels}, which moves to a new image without a commit"
            why = pending_reason(pending, name, job_id)
            if why:
                notes.append(f"{name}: job '{job_id}' {problem} (pending: {why})")
            else:
                errors.append(f"{name}: job '{job_id}' {problem}")
    return errors, notes


def version_key(text):
    match = re.fullmatch(r"(\d+)\.(\d+)(?:\.(\d+))?", str(text))
    if not match:
        return None
    return (int(match[1]), int(match[2]), int(match[3] or 0))


def same_version(a, b):
    key = version_key(a)
    return key is not None and key == version_key(b)


def msrv_problems(root, matrix_expr, manifests, deliberate):
    """Where the MSRV differs from `root`: the matrix strings of the test job, then each crate."""
    problems = []
    arrays = re.findall(r"'(\[[^']*\])'", matrix_expr or "")
    if not arrays:
        problems.append("ci.yml: no matrix string (a JSON array in single quotes) in the test job's include")
    for index, text in enumerate(arrays, 1):
        where = f"ci.yml: test matrix string {index}"
        try:
            entries = json.loads(text)
        except ValueError:
            problems.append(f"{where} is not JSON")
            continue
        found = [e for e in entries if isinstance(e, dict) and e.get("label") == "msrv"]
        if len(found) != 1:
            problems.append(f"{where} has {len(found)} entries labelled msrv, not one")
        elif not same_version(found[0].get("rust"), root):
            problems.append(f"{where}: the msrv entry names rust {found[0].get('rust')!r}, Cargo.toml's rust-version is {root!r}")
    for path, version in sorted(manifests.items()):
        if version is None:
            continue
        if path in deliberate:
            if same_version(version, root):
                problems.append(f"{path}: rust-version {version!r} no longer differs from the workspace's; remove it from DELIBERATE_MSRV")
            elif not same_version(version, deliberate[path]):
                problems.append(f"{path}: rust-version {version!r}, expected the deliberate {deliberate[path]!r}")
        elif not same_version(version, root):
            problems.append(f"{path}: rust-version {version!r}, Cargo.toml's is {root!r}")
    for path in sorted(deliberate):
        if manifests.get(path) is None:
            problems.append(f"{path}: named in DELIBERATE_MSRV but declares no rust-version")
    return problems


def glob_rule(glob):
    """A GitHub `paths-ignore` pattern as (kind, value); ValueError for a shape not read here.

    `LICENSE*` matches at the root only and `^LICENSE` as a regex also matches a directory of
    that name; both read as a prefix, since no such directory exists.
    """
    wild = set("*?[]!")
    if not glob or glob.startswith("!"):
        raise ValueError(f"cannot read the path pattern {glob!r}")
    if glob.startswith("**"):
        rest = glob[2:]
        if rest and not (wild | {"/"}) & set(rest):
            return ("suffix", rest)
    elif glob.endswith("/**"):
        rest = glob[:-3]
        if rest and not wild & set(rest):
            return ("prefix", rest + "/")
    elif glob.endswith("*"):
        rest = glob[:-1]
        if rest and not (wild | {"/"}) & set(rest):
            return ("prefix", rest)
    elif not wild & set(glob):
        return ("exact", glob)
    raise ValueError(f"cannot read the path pattern {glob!r}")


def regex_rule(regex):
    """One alternative of a `grep -E` pattern as (kind, value); ValueError for a shape not read here."""
    body = regex
    start = body.startswith("^")
    if start:
        body = body[1:]
    end = body.endswith("$") and not body.endswith("\\$")
    if end:
        body = body[:-1]
    if not body or not re.fullmatch(r"(?:[A-Za-z0-9_/-]|\\\.)+", body) or not (start or end):
        raise ValueError(f"cannot read the pattern {regex!r}")
    value = body.replace("\\.", ".")
    return ("exact" if start and end else "prefix" if start else "suffix", value)


def covers(a, b):
    (kind_a, value_a), (kind_b, value_b) = a, b
    if a == b:
        return False
    if kind_a == "suffix":
        return kind_b in ("suffix", "exact") and value_b.endswith(value_a)
    if kind_a == "prefix":
        return kind_b in ("prefix", "exact") and value_b.startswith(value_a)
    return False


def normalise(rules):
    """The rules without any that another rule already covers (a redundant `CHANGELOG.md`)."""
    return {r for r in rules if not any(covers(o, r) for o in rules)}


def matches(rule, path):
    kind, value = rule
    return path.endswith(value) if kind == "suffix" else path.startswith(value) if kind == "prefix" else path == value


def show(rules):
    return ", ".join(f"{kind} {value}" for kind, value in sorted(rules)) or "none"


GREP_GROUP = re.compile(r"grep -q?vE '\(([^']*)\)'")
BUILT_FROM = re.compile(r"built_from=\$\(grep -E '([^']*)'")
BUILT_FILES = re.compile(r"\^(?P<dir>[A-Za-z0-9_./-]+/)\((?P<names>[A-Za-z0-9_|-]+)\)\\\.(?P<ext>[A-Za-z0-9]+)\$")


def group_of(run, where):
    found = GREP_GROUP.findall(run or "")
    if len(found) != 1:
        return None, f"{where}: expected one `grep -vE '(...)'` documentation pattern, found {len(found)}"
    return found[0], None


def docs_problems(paths_ignore, viewer_run, deploy_run, expected_built_from):
    """Where the three documentation-only path lists differ."""
    problems = []
    reference = None
    try:
        reference = normalise({glob_rule(g) for g in paths_ignore})
    except ValueError as error:
        problems.append(f"ci.yml paths-ignore: {error}")
    groups = {}
    for where, run in (("ci.yml viewer-paths", viewer_run), ("deploy-viewer.yml wait step", deploy_run)):
        group, error = group_of(run, where)
        if error:
            problems.append(error)
            continue
        groups[where] = group
        try:
            rules = normalise({regex_rule(alt) for alt in group.split("|")})
        except ValueError as error:
            problems.append(f"{where}: {error}")
            continue
        if reference is not None and rules != reference:
            parts = []
            if rules - reference:
                parts.append(f"counts {show(rules - reference)} as documentation, which ci.yml's push paths-ignore does not")
            if reference - rules:
                parts.append(f"does not count {show(reference - rules)} as documentation, which ci.yml's push paths-ignore does")
            problems.append(f"{where} {' and '.join(parts)}")
    if len(set(groups.values())) > 1:
        problems.append("ci.yml viewer-paths and deploy-viewer.yml's wait step must carry the same documentation regex")
    built = BUILT_FROM.findall(viewer_run or "")
    if len(built) != 1:
        problems.append(f"ci.yml viewer-paths: expected one built_from pattern, found {len(built)}")
        return problems
    match = BUILT_FILES.fullmatch(built[0])
    if not match:
        problems.append(f"ci.yml viewer-paths: cannot read the built_from pattern {built[0]!r}")
        return problems
    files = {f"{match['dir']}{name}.{match['ext']}" for name in match["names"].split("|")}
    if files != set(expected_built_from):
        problems.append(
            f"ci.yml viewer-paths runs the viewer tests for {sorted(files)}, not the deliberate {sorted(expected_built_from)}"
        )
    if reference is not None:
        for path in sorted(files):
            if not any(matches(rule, path) for rule in reference):
                problems.append(f"ci.yml viewer-paths carves out {path}, which paths-ignore does not count as documentation")
    return problems


def workflow_on(doc):
    """The `on:` block; YAML 1.1 reads the key `on` as true."""
    return doc.get("on", doc.get(True)) or {}


def step_run(job, **want):
    for step in job.get("steps", []):
        if all(step.get(k) == v for k, v in want.items()):
            return step.get("run")
    return None


def tracked_rust_versions():
    out = subprocess.run(
        ["git", "ls-files", "-z", "--", "*Cargo.toml"], cwd=ROOT, capture_output=True, text=True, check=True
    ).stdout
    versions = {}
    for path in sorted(p for p in out.split("\0") if p):
        package = tomllib.loads((ROOT / path).read_text()).get("package", {})
        version = package.get("rust-version")
        versions[path] = version if isinstance(version, str) else None
    return versions


def main():
    import yaml

    workflows = {p.name: yaml.safe_load(p.read_text()) for p in sorted(WORKFLOWS.glob("*.yml"))}
    errors, notes = [], []

    found, pending = timeout_problems(workflows, PENDING)
    errors += found
    notes += pending
    found, pending = label_problems(workflows, PENDING)
    errors += found
    notes += pending
    jobs = [j for d in workflows.values() for j in (d.get("jobs") or {}).values() if "uses" not in j and "steps" in j]
    print(f"timeouts and runner labels: {len(jobs)} jobs run steps in {len(workflows)} workflows, {len(notes)} pending notes")

    manifests = tracked_rust_versions()
    root = manifests.get("Cargo.toml")
    ci = workflows["ci.yml"]
    include = ci["jobs"]["test"]["strategy"]["matrix"]["include"]
    if root is None:
        errors.append("Cargo.toml: no package rust-version")
    else:
        errors += msrv_problems(root, include if isinstance(include, str) else "", manifests, DELIBERATE_MSRV)
        print(f"msrv: rust-version {root} in Cargo.toml, {sum(v is not None for v in manifests.values())} crates, ci.yml's test matrix")

    deploy = workflows["deploy-viewer.yml"]["jobs"]["deploy"]
    errors += docs_problems(
        workflow_on(ci).get("push", {}).get("paths-ignore", []),
        step_run(ci["jobs"]["viewer-paths"], id="paths"),
        step_run(deploy, name="Wait for CI on the commit"),
        EXPECTED_BUILT_FROM,
    )
    print("documentation paths: ci.yml paths-ignore, viewer-paths and Deploy viewer's wait step")

    for error in errors:
        print(annotation("error", error))
    for note in notes:
        print(annotation("warning", note))
    print(f"{len(errors)} errors, {len(notes)} warnings")
    return 1 if errors else 0


def annotation(level, message):
    """A GitHub log annotation, on the file the message starts with."""
    head = message.split(":", 1)[0]
    if re.fullmatch(r"[\w.-]+\.yml", head):
        return f"::{level} file=.github/workflows/{head}::{message}"
    if head.endswith("Cargo.toml"):
        return f"::{level} file={head}::{message}"
    return f"::{level}::{message}"


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Move every version file to a new release version in one step (RELEASE.md, step 4).

    python3 tools/release/bump_version.py X.Y.Z [--date YYYY-MM-DD]
    python3 tools/release/bump_version.py X.Y.Z --release-workflows-from origin/main

The files and their checks are `tools/ci/release_preflight.py`'s, so the two
cannot drift: each crate's own version, each exact `=X.Y.Z` pin on `rvt`, the
workspace crates' entries in `Cargo.lock` and `fuzz/Cargo.lock`, the viewer's
`package.json`, and `CITATION.cff`'s `version:` and `date-released:` (today,
or `--date`). Only the version text in those places changes, nothing else in
the files. The preflight's own check then reads the tree back. A version file
left behind fails the bump, and so do version files that disagree before it.
The `CHANGELOG.md` section is the one thing it leaves: it says so if the section
is missing. `python3 tools/release/changelog.py assemble` and the section's
introduction are steps 5 and 6.

`--release-workflows-from REF` is for a hotfix cut from an older tag (RELEASE.md,
"Hotfix process", step 6). A tag runs the workflows as they are at the tagged
commit. So it copies REF's `publish.yml`, `release-binaries.yml`, the actions
they use and `release_preflight.py` onto the branch, as 0.14.1's hotfix did by
hand.

`--root DIR` bumps another checkout, such as a hotfix worktree cut from a tag
older than this script: the preflight's lists and check are this checkout's.
"""

import argparse
import datetime
import importlib.util
import pathlib
import re
import subprocess
import sys

SCRIPT_ROOT = pathlib.Path(__file__).resolve().parents[2]
ROOT = SCRIPT_ROOT
RELEASE_WORKFLOW_FILES = (
    ".github/workflows/publish.yml",
    ".github/workflows/release-binaries.yml",
    ".github/actions/setup-rust/action.yml",
    ".github/actions/wait-for-check/action.yml",
    ".github/actions/wait-for-check/wait.sh",
    ".github/actions/wasm-tools/action.yml",
    "tools/ci/release_preflight.py",
)


def preflight():
    spec = importlib.util.spec_from_file_location("release_preflight", SCRIPT_ROOT / "tools/ci/release_preflight.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def read_files(pf):
    files = {}
    for path in pf.FILES:
        try:
            files[path] = (ROOT / path).read_text()
        except OSError:
            files[path] = None
    return files


def replace_once(text, pattern, replacement, what):
    new, count = re.subn(pattern, replacement, text, count=1, flags=re.M)
    if count != 1:
        sys.exit(f"{what}: the version text was not found where the preflight reads it")
    return new


def bump(pf, old, new, date):
    changed = []
    for path, name, owns_version, pins_rvt in pf.MANIFESTS:
        text = (ROOT / path).read_text()
        before = text
        if owns_version:
            # The first `version = "old"` after `[package]`: the package's own.
            head, sep, rest = text.partition("[package]")
            if not sep:
                sys.exit(f"{path} has no [package] table")
            rest = replace_once(rest, rf'^version = "{re.escape(old)}"$', f'version = "{new}"', f"{path} version")
            text = head + sep + rest
        if pins_rvt:
            text = replace_once(text, rf'^(rvt = \{{[^}}\n]*version = "=){re.escape(old)}(")', rf"\g<1>{new}\g<2>",
                                f"{path} pin on rvt")
        if text != before:
            (ROOT / path).write_text(text)
            changed.append(path)
    for path, names in pf.LOCKS.items():
        text = (ROOT / path).read_text()
        for name in names:
            # A workspace entry has no `source` line after its version.
            text = replace_once(text, rf'^(name = "{re.escape(name)}"\nversion = "){re.escape(old)}("\n(?!source))',
                                rf"\g<1>{new}\g<2>", f"{path} entry for {name}")
        (ROOT / path).write_text(text)
        changed.append(path)
    path = "viewer/package.json"
    text = (ROOT / path).read_text()
    (ROOT / path).write_text(replace_once(text, rf'^(  "version": "){re.escape(old)}(",)$', rf"\g<1>{new}\g<2>", path))
    changed.append(path)
    path = "CITATION.cff"
    text = (ROOT / path).read_text()
    text = replace_once(text, rf"^version: {re.escape(old)}$", f"version: {new}", f"{path} version")
    text = replace_once(text, r'^date-released: "\d{4}-\d{2}-\d{2}"$', f'date-released: "{date}"', f"{path} date")
    (ROOT / path).write_text(text)
    changed.append(path)
    return changed


def copy_release_workflows(ref):
    for path in RELEASE_WORKFLOW_FILES:
        blob = subprocess.run(["git", "show", f"{ref}:{path}"], cwd=ROOT, capture_output=True)
        if blob.returncode != 0:
            sys.exit(f"{ref} has no {path}")
        target = ROOT / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(blob.stdout)
        print(f"copied {path} from {ref}")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("version")
    ap.add_argument("--date", default=datetime.date.today().isoformat())
    ap.add_argument("--release-workflows-from", metavar="REF")
    ap.add_argument("--root", type=pathlib.Path, help="the checkout to bump (default: this one)")
    a = ap.parse_args()
    global ROOT
    if a.root:
        ROOT = a.root.resolve()
    pf = preflight()
    if not pf.SEMVER.fullmatch(a.version):
        sys.exit(f"{a.version!r} is not a semver version")
    if not re.fullmatch(r"\d{4}-\d{2}-\d{2}", a.date):
        sys.exit(f"--date {a.date!r} is not YYYY-MM-DD")
    old, problems = pf.check_versions(read_files(pf), None)
    problems = [p for p in problems if "CHANGELOG.md" not in p]
    if old is None or problems:
        sys.exit("the version files disagree before the bump:\n  " + "\n  ".join(problems))
    if old == a.version:
        sys.exit(f"the version files already say {old}")
    for path in bump(pf, old, a.version, a.date):
        print(f"{old} -> {a.version}: {path}")
    if a.release_workflows_from:
        copy_release_workflows(a.release_workflows_from)
    version, problems = pf.check_versions(read_files(pf), a.version)
    missing_section = [p for p in problems if "CHANGELOG.md" in p]
    problems = [p for p in problems if p not in missing_section]
    if problems:
        sys.exit("the preflight disagrees after the bump:\n  " + "\n  ".join(problems))
    print(f"every version file says {a.version}; CITATION.cff is dated {a.date}")
    for p in missing_section:
        print(f"still to do: {p} (RELEASE.md, steps 5 and 6)")


if __name__ == "__main__":
    main()

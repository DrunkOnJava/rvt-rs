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
commit. So it copies REF's `publish.yml`, `release-binaries.yml`, every local
action they use (read from the workflows themselves, with the files of its
folder, and from those actions in turn) and `release_preflight.py` onto the
branch, as 0.14.1's hotfix did by hand. REF must be a commit `origin/main`
holds: the workflows are copied as `main` has them, not from a branch no one
has reviewed. It then reads the copied preflight and refuses if the version
files it reads are not the ones this bump moved.

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
# What a hotfix branch carries from main; the local actions the workflows use are found in them.
RELEASE_WORKFLOW_FILES = (
    ".github/workflows/publish.yml",
    ".github/workflows/release-binaries.yml",
    "tools/ci/release_preflight.py",
)
# A local action, in either form a workflow names it: `./.github/actions/NAME` or `$/.github/actions/NAME`.
LOCAL_ACTION = re.compile(r"^[ \t]*(?:-[ \t]*)?uses:[ \t]*(?:\./|\$/)(\.github/actions/[A-Za-z0-9_.-]+)[ \t]*(?:#.*)?$", re.M)


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
            rest = replace_once(rest, rf'^(version[ \t]*=[ \t]*"){re.escape(old)}(")', rf"\g<1>{new}\g<2>", f"{path} version")
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
    # Any indentation, and the last key of the object too: the preflight reads the file as JSON.
    (ROOT / path).write_text(replace_once(text, rf'^([ \t]*"version"[ \t]*:[ \t]*"){re.escape(old)}(")', rf"\g<1>{new}\g<2>", path))
    changed.append(path)
    path = "CITATION.cff"
    (ROOT / path).write_text(bump_citation((ROOT / path).read_text(), pf, old, new, date))
    changed.append(path)
    return changed


def bump_citation(text, pf, old, new, date):
    """CITATION.cff with its `version:` and `date-released:` moved. The version line is found with the preflight's own
    pattern, so every form the preflight reads (quoted, with a comment) is one this rewrites."""
    found = list(pf.CITATION_VERSION.finditer(text))
    if len(found) != 1 or found[0].group(1) != old:
        sys.exit(f"CITATION.cff version: the version text was not found where the preflight reads it ({len(found)} lines)")
    text = text[: found[0].start(1)] + new + text[found[0].end(1) :]
    return replace_once(text, r'^date-released:[ \t]*"?\d{4}-\d{2}-\d{2}"?[ \t]*(?:#.*)?$', f'date-released: "{date}"', "CITATION.cff date")


def valid_date(text):
    """Whether `text` is a calendar date written YYYY-MM-DD: 2026-13-45 has the shape and is not one."""
    if not re.fullmatch(r"[0-9]{4}-[0-9]{2}-[0-9]{2}", text):
        return False
    try:
        datetime.date.fromisoformat(text)
    except ValueError:
        return False
    return True


def git(*args):
    return subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True)


def main_commit(ref):
    """The commit `ref` names, which must be one `origin/main` holds (RELEASE.md copies from `main`)."""
    named = git("rev-parse", "--verify", "--quiet", f"{ref}^{{commit}}")
    if named.returncode != 0:
        sys.exit(f"--release-workflows-from {ref!r} is not a commit of this checkout")
    commit = named.stdout.strip()
    if git("rev-parse", "--verify", "--quiet", "origin/main").returncode != 0:
        sys.exit("origin/main is not in this checkout, so whether --release-workflows-from is on main cannot be told: git fetch origin main")
    if git("merge-base", "--is-ancestor", commit, "origin/main").returncode != 0:
        sys.exit(
            f"--release-workflows-from {ref!r} ({commit[:12]}) is not on origin/main: the release workflows are copied "
            "as main has them, not from a branch or a tag no one has reviewed for this"
        )
    return commit


def local_actions(text):
    """The action folders (`.github/actions/NAME`) a workflow or action file names, in the order it names them."""
    return list(dict.fromkeys(LOCAL_ACTION.findall(text)))


def copy_release_workflows(ref, commit):
    queue = list(RELEASE_WORKFLOW_FILES)
    folders = []
    copied = []
    while queue:
        path = queue.pop(0)
        blob = git_bytes("show", f"{commit}:{path}")
        if blob is None:
            sys.exit(f"{ref} ({commit[:12]}) has no {path}")
        target = ROOT / path
        existed = target.read_bytes() if target.is_file() else None
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(blob)
        what = "new" if existed is None else "identical" if existed == blob else "replaces a file that differs"
        print(f"copied {path} from {ref} ({commit[:12]}): {what}")
        copied.append(path)
        if path.endswith((".yml", ".yaml")):
            for folder in local_actions(blob.decode("utf-8", errors="replace")):
                if folder not in folders:
                    folders.append(folder)
                    listed = git("ls-tree", "-r", "--name-only", commit, folder)
                    files = [line for line in listed.stdout.split("\n") if line]
                    if listed.returncode != 0 or not files:
                        sys.exit(f"{ref} ({commit[:12]}) has no {folder}, which {path} uses")
                    queue.extend([name for name in files if name not in copied and name not in queue])


def git_bytes(*args):
    done = subprocess.run(["git", *args], cwd=ROOT, capture_output=True)
    return done.stdout if done.returncode == 0 else None


def copied_preflight_differs(pf):
    """The version-file lists the copied preflight reads that are not the ones this bump moves, by name."""
    spec = importlib.util.spec_from_file_location("release_preflight_copy", ROOT / "tools/ci/release_preflight.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return [name for name in ("FILES", "MANIFESTS", "LOCKS") if getattr(module, name, None) != getattr(pf, name)]


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
    if not valid_date(a.date):
        sys.exit(f"--date {a.date!r} is not a calendar date written YYYY-MM-DD")
    old, problems = pf.check_versions(read_files(pf), None)
    problems = [p for p in problems if "CHANGELOG.md" not in p]
    if old is None or problems:
        sys.exit("the version files disagree before the bump:\n  " + "\n  ".join(problems))
    if old == a.version:
        sys.exit(f"the version files already say {old}")
    # Before anything is written: a ref that is refused leaves the checkout as it was.
    commit = main_commit(a.release_workflows_from) if a.release_workflows_from else None
    for path in bump(pf, old, a.version, a.date):
        print(f"{old} -> {a.version}: {path}")
    if a.release_workflows_from:
        copy_release_workflows(a.release_workflows_from, commit)
        differs = copied_preflight_differs(pf)
        if differs:
            sys.exit(
                "the copied tools/ci/release_preflight.py reads other version files than this bump moves "
                f"({', '.join(differs)} differ): copy the workflows from a main this script is from"
            )
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

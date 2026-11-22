#!/usr/bin/env python3
"""The small scored outputs of a Measure run, kept past the 90 days GitHub keeps its artifacts (#2124).

The evidence pages cite Measure runs by id, and a run's artifacts expire 90 days after it ran. The
citations read scorer outputs on either side, export digests, export diagnostics' counts, timings,
and the run's verdict. `<source>` holds a run's artifacts as `gh run download -n <name> -D
<source>/<name>` leaves them, and this copies into `<out>`:

- `head/` and `base/`, from `measure-head` and `measure-base`, model by model: every scorer output
  (`.txt`, but not a probe's own output, `probe.txt`, `probe-<probe>.txt` or `probe-pair*.txt`, which
  can be large and holds a file's coordinates), `ifc.sha256`, `diagnostics.json`, `timings.log` and
  the witness observations (`witness-*.json`), and the artifact's `commit.txt`;
- `verdict/` and `compare/`, the whole of `measure-verdict` and `compare-scorer-diff`.

Models are kept only from the folders Measure gives the models anyone can fetch: `core-interior`,
`einhoven`, `re1-*`, `nordic-*`, `r2025-*`, `sample-*`, `family-*`, and a `spent-*` pair that
research/holdout-archive-models.tsv still lists as spent and no held-out list names. The held-out,
`rac_advanced` and BIMcollab models' folders (`holdout-*`, `racadv-*`, `bimcollab-*`), whose outputs
Measure keeps to summary counts, and any other `spent-*` folder are left out. Any other folder
stops the run, so a new kind of model is looked at before it is kept. Logs, the runner's own files,
the exports, `info.json` (a model's own metadata, which can name the path it was saved at) and MCP
dumps are left out.

It writes `KEPT.md` saying what was kept and from where, then scans everything it wrote with the CI
PII guard's patterns and fails on a match, since a kept copy is public once pushed.

    python3 tools/ci/keep-run-outputs.py <source> <out> --run <run id> [--root <repository>]
"""

import argparse
import os
import re
import shutil
import sys

MODEL_SIDES = {"measure-head": "head", "measure-base": "base"}
WHOLE = {"measure-verdict": "verdict", "compare-scorer-diff": "compare"}
OPEN_FOLDERS = ("core-interior", "einhoven")
OPEN_PREFIXES = ("re1-", "nordic-", "r2025-", "sample-", "family-")
LEFT_OUT_PREFIXES = ("holdout-", "racadv-", "bimcollab-")
PROBE_OUTPUT = re.compile(r"probe(-pair)?(-.+)?\.txt")
# The CI PII guard's patterns (.github/workflows/ci.yml, "PII guard"), as Python regexes.
PII = [
    r"/Users/[A-Za-z][A-Za-z0-9._-]+/",
    r"/home/[A-Za-z][A-Za-z0-9._-]+/",
    r"C:\\Users\\[A-Za-z][A-Za-z0-9._ -]+\\",
    r"OneDrive - Autodesk\\[^\\]*\\Revit - [0-9]{3,}",
    r"F:\\Ship\\[0-9]{4}_",
]


def stem(path):
    return os.path.splitext(os.path.basename(path))[0]


def spent_stems(root):
    """(stems listed as spent, stems any held-out list names)."""
    spent, held = set(), set()
    with open(os.path.join(root, "research/holdout-models.tsv"), encoding="utf-8") as handle:
        for line in handle:
            if line.strip() and not line.startswith("#"):
                held.add(stem(line.split("\t")[0]))
    with open(os.path.join(root, "research/holdout-archive-models.tsv"), encoding="utf-8") as handle:
        for line in handle:
            cells = line.rstrip("\n").split("\t")
            if len(cells) > 1 and cells[0] == "member":
                (spent if cells[1].startswith("spent/") else held).add(stem(cells[1]))
    return spent, held


def kept_file(name):
    if PROBE_OUTPUT.fullmatch(name):
        return False
    return (name.endswith(".txt") or name in ("ifc.sha256", "diagnostics.json", "timings.log")
            or (name.startswith("witness-") and name.endswith(".json")))


def keep_models(source, out, spent, held, notes):
    """Copy one side's kept files; return (models kept, files kept)."""
    models = files = 0
    for top in sorted(os.listdir(source)):
        path = os.path.join(source, top)
        if not os.path.isdir(path):
            if top == "commit.txt":
                os.makedirs(out, exist_ok=True)
                shutil.copyfile(path, os.path.join(out, top))
                files += 1
            continue
        if top.startswith(LEFT_OUT_PREFIXES):
            notes.add(f"the {top.split('-', 1)[0]} models' folders, whose outputs Measure keeps to summary counts")
            continue
        if top.startswith("spent-"):
            name = top[len("spent-"):]
            if name not in spent or name in held:
                notes.add(f"`{top}`, which no list names as spent, or a held-out list names")
                continue
        elif top not in OPEN_FOLDERS and not top.startswith(OPEN_PREFIXES):
            sys.exit(f"{source}: a folder no rule here knows, {top}: say whether its model is one anyone can fetch first")
        held_any = False
        for directory, _, names in os.walk(path):
            for name in sorted(names):
                if not kept_file(name):
                    continue
                rel = os.path.relpath(os.path.join(directory, name), source)
                target = os.path.join(out, rel)
                os.makedirs(os.path.dirname(target), exist_ok=True)
                shutil.copyfile(os.path.join(directory, name), target)
                files += 1
                held_any = True
        models += held_any
    return models, files


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("source")
    parser.add_argument("out")
    parser.add_argument("--run", required=True)
    parser.add_argument("--root", default=os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9]+", args.run):
        sys.exit("--run must be a run id")
    if os.path.exists(args.out) and os.listdir(args.out):
        sys.exit(f"{args.out} is not empty")
    os.makedirs(args.out, exist_ok=True)
    spent, held = spent_stems(args.root)
    notes, lines = set(), []
    for artifact, side in MODEL_SIDES.items():
        path = os.path.join(args.source, artifact)
        if os.path.isdir(path):
            models, files = keep_models(path, os.path.join(args.out, side), spent, held, notes)
            lines.append(f"- `{side}/`, from `{artifact}`: {files} files for {models} models")
    for artifact, side in WHOLE.items():
        path = os.path.join(args.source, artifact)
        if os.path.isdir(path):
            shutil.copytree(path, os.path.join(args.out, side))
            count = sum(len(f) for _, _, f in os.walk(os.path.join(args.out, side)))
            lines.append(f"- `{side}/`, the whole of `{artifact}`: {count} file{'' if count == 1 else 's'}")
    if not lines:
        sys.exit(f"{args.source} holds none of {', '.join(list(MODEL_SIDES) + list(WHOLE))}")
    commit = ""
    for side in ("head", "base"):
        commit_file = os.path.join(args.out, side, "commit.txt")
        if not commit and os.path.isfile(commit_file):
            with open(commit_file, encoding="utf-8", errors="replace") as handle:
                commit = (handle.read().split() or [""])[0]
    with open(os.path.join(args.out, "KEPT.md"), "w", encoding="utf-8") as handle:
        handle.write(
            f"# Measure run {args.run}: kept outputs\n\n"
            f"From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/{args.run}"
            + (f", whose head side measured `{commit}`" if commit else "")
            + ", by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:\n\n"
            + "\n".join(lines) + "\n\n"
            "Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps "
            "and the probes' own output" + "".join(f"; {note}" for note in sorted(notes)) + ".\n"
        )
    patterns = [re.compile(p) for p in PII]
    hits = []
    for directory, _, names in os.walk(args.out):
        for name in names:
            with open(os.path.join(directory, name), encoding="utf-8", errors="replace") as handle:
                text = handle.read()
            hits += [(os.path.relpath(os.path.join(directory, name), args.out), p.pattern) for p in patterns if p.search(text)]
    print("\n".join(lines))
    if hits:
        for path, pattern in hits:
            print(f"PII-shaped pattern {pattern} in {path}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())

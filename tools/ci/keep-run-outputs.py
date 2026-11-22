#!/usr/bin/env python3
"""The small scored outputs of a Measure run, kept past the 90 days GitHub keeps its artifacts (#2124).

The evidence pages cite Measure runs by id, and a run's artifacts expire 90 days after it ran.
This copies, from an unpacked `measure-head` artifact, what the citations read:

- each model's scorer outputs (every `.txt` but the probe's own `probe.txt`, which can be large and
  holds a file's coordinates) and its `ifc.sha256`, at their paths in the artifact;
- for each `diagnostics.json`, its sha256 in `diagnostics.json.sha256`, so a later export can be
  checked against it without the file;
- the artifact's `commit.txt`, the commit Measure measured.

It leaves out every log, the runner's own files (`runner-*`, `vmstat-*`, `score-cache-*`) and the
folders of the held-out, `rac_advanced` and BIMcollab models (`holdout-*`, `racadv-*`,
`bimcollab-*`), whose outputs Measure keeps to summary counts and some of whose runs were deleted
on purpose. It writes `KEPT.md` saying what was kept and from where, then scans everything it wrote
with the CI PII guard's patterns and fails on a match, since a kept copy is public once pushed.

    python3 tools/ci/keep-run-outputs.py <unpacked measure-head> <out dir> --run <run id> [--artifact <id>]
"""

import argparse
import hashlib
import os
import re
import shutil
import sys

LEFT_OUT_FOLDERS = ("holdout-", "racadv-", "bimcollab-")
# The CI PII guard's patterns (.github/workflows/ci.yml, "PII guard"), as Python regexes.
PII = [
    r"/Users/[A-Za-z][A-Za-z0-9._-]+/",
    r"/home/[A-Za-z][A-Za-z0-9._-]+/",
    r"C:\\Users\\[A-Za-z][A-Za-z0-9._ -]+\\",
    r"OneDrive - Autodesk\\[^\\]*\\Revit - [0-9]{3,}",
    r"F:\\Ship\\[0-9]{4}_",
]


def kept(name):
    return name == "ifc.sha256" or (name.endswith(".txt") and name != "probe.txt")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("source")
    parser.add_argument("out")
    parser.add_argument("--run", required=True)
    parser.add_argument("--artifact", default="")
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9]+", args.run):
        sys.exit("--run must be a run id")
    if os.path.exists(args.out) and os.listdir(args.out):
        sys.exit(f"{args.out} is not empty")
    os.makedirs(args.out, exist_ok=True)
    counts = {"scorer outputs": 0, "ifc.sha256": 0, "diagnostics digests": 0}
    models, left_out = set(), set()
    commit = ""
    for top in sorted(os.listdir(args.source)):
        path = os.path.join(args.source, top)
        if not os.path.isdir(path):
            if top == "commit.txt":
                shutil.copyfile(path, os.path.join(args.out, top))
                with open(path, encoding="utf-8", errors="replace") as handle:
                    commit = handle.read().strip()
            continue
        if top.startswith(LEFT_OUT_FOLDERS):
            left_out.add(top.split("-", 1)[0])
            continue
        for root, _, files in os.walk(path):
            for name in sorted(files):
                rel = os.path.relpath(os.path.join(root, name), args.source)
                target = os.path.join(args.out, rel)
                if name == "diagnostics.json":
                    with open(os.path.join(root, name), "rb") as handle:
                        digest = hashlib.sha256(handle.read()).hexdigest()
                    os.makedirs(os.path.dirname(target), exist_ok=True)
                    with open(target + ".sha256", "w", encoding="utf-8") as handle:
                        handle.write(f"{digest}  diagnostics.json\n")
                    counts["diagnostics digests"] += 1
                elif kept(name):
                    os.makedirs(os.path.dirname(target), exist_ok=True)
                    shutil.copyfile(os.path.join(root, name), target)
                    counts["ifc.sha256" if name == "ifc.sha256" else "scorer outputs"] += 1
                else:
                    continue
                models.add(top)
    with open(os.path.join(args.out, "KEPT.md"), "w", encoding="utf-8") as handle:
        handle.write(
            f"# Measure run {args.run}: kept outputs\n\n"
            f"From the `measure-head` artifact{f' {args.artifact}' if args.artifact else ''} of "
            f"https://github.com/DrunkOnJava/rvt-rs/actions/runs/{args.run}, which measured "
            f"`{commit.split()[0] if commit else 'an unrecorded commit'}`, by `tools/ci/keep-run-outputs.py` (#2124).\n\n"
            f"Kept, for {len(models)} models: {counts['scorer outputs']} scorer outputs, {counts['ifc.sha256']} `ifc.sha256`, "
            f"and {counts['diagnostics digests']} sha256 digests of `diagnostics.json`; and `commit.txt`.\n\n"
            "Left out: the probes' own `probe.txt`, every log, the runner's own files, and the exports themselves"
            + (f"; and the folders of {', '.join(sorted(left_out))} models, whose outputs Measure keeps to summary counts" if left_out else "")
            + ".\n"
        )
    hits = []
    patterns = [re.compile(p) for p in PII]
    for root, _, files in os.walk(args.out):
        for name in files:
            with open(os.path.join(root, name), encoding="utf-8", errors="replace") as handle:
                text = handle.read()
            hits += [(os.path.relpath(os.path.join(root, name), args.out), p.pattern) for p in patterns if p.search(text)]
    print(f"kept {sum(counts.values())} files for {len(models)} models: " + ", ".join(f"{k} {v}" for k, v in counts.items())
          + (f"; left out {', '.join(sorted(left_out))} folders" if left_out else ""))
    if hits:
        for path, pattern in hits:
            print(f"PII-shaped pattern {pattern} in {path}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())

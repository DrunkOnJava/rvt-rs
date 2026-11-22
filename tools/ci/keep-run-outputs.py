#!/usr/bin/env python3
"""The small scored outputs of a Measure run, kept past the 90 days GitHub keeps its artifacts (#2124).

The evidence pages cite Measure runs by id, and a run's artifacts expire 90 days after it ran. The
citations read scorer outputs on either side, export digests, export diagnostics' counts, timings,
and the run's verdict. `<source>` holds a run's artifacts as `gh run download -n <name> -D
<source>/<name>` leaves them, and this copies into `<out>`:

- `head/` and `base/`, from `measure-head` and `measure-base`, model by model: every scorer output
  (`.txt`, but not a probe's own output, `probe.txt`, `probe-<probe>.txt` or `probe-pair*.txt`, which
  can be large and holds a file's coordinates), `ifc.sha256`, `diagnostics.json`, `timings.log` and
  the witness observations (`witness-*.json`), and at the artifact's top the outputs that read
  several models (`areas_across_saves.txt` and the like) and its `commit.txt`;
- `verdict/`, `compare/` and `timing/`, the whole of `measure-verdict`, `compare-scorer-diff` and
  `measure-timing` (each model's export time and peak memory on both sides), except that the scorer
  diff's sections for the models whose folders are left out (below) are left out too.

Models are kept only from the folders Measure gives the models anyone can fetch: `core-interior`,
`einhoven`, `re1-*`, `nordic-*`, `r2025-*`, `sample-*`, `family-*`, and a `spent-*` pair that
research/holdout-archive-models.tsv still lists as spent and no held-out list names. The held-out,
`rac_advanced` and BIMcollab models' folders (`holdout-*`, `racadv-*`, `bimcollab-*`), whose outputs
Measure keeps to summary counts, and any other `spent-*` folder are left out. Any other folder
stops the run, so a new kind of model is looked at before it is kept. Logs, the runner's own files,
the exports, `info.json` (a model's own metadata, which can name the path it was saved at) and MCP
dumps are left out.

With `--part probes` it keeps instead, from the same folders and under the same scans, only the
probes' own output and `commit.txt`, for a commit beside the run's first: a figure that only a probe
prints is cited from it, and the first kept commit, which the citations already link to, never moves.

With `--part holdout` it keeps only each held-out model's `counts.txt`, the summary counts
Measure keeps of it, and `commit.txt`, for a commit beside the run's first, so that the held-out
figures the evidence pages cite keep a source (Griffin, 2026-11-22). Their folders' other files,
and the `rac_advanced` and BIMcollab folders, stay out, and the folder-name scan below allows
the held-out models' names for this part alone.

With `--part log --lines <pattern>` it keeps the lines of a run's log that `<pattern>` (a Python
regular expression, searched in each line's text) finds, for a figure a run prints only to its log,
such as a viewer test's measurement (#2148). `<source>/run.log` holds the log as `gh run view --log`
prints it, one `<job>\t<step>\t<time> <text>` line per log line; `log/lines.txt` keeps
`<job>\t<time>\t<text>` for each line found, with terminal colour codes taken out. A pattern that
finds nothing stops the run, as does a run not of this repository or of a pull request, which the
workflow refuses before this runs.

It writes `KEPT.md` saying what was kept and from where, then scans everything it wrote and fails on
a match, since a kept copy is public once pushed: the CI PII guard's patterns, read a second time
with JSON's doubled backslashes made single (a Windows path in `diagnostics.json` is written
`C:\\Users\\...`), and the name of any folder left out. The timing table's rows name files, not
folders, so its rows for a model a held-out list names are left out by name.

    python3 tools/ci/keep-run-outputs.py <source> <out> --run <run id> [--part outputs|probes|holdout|log] [--lines <pattern>] [--root <repository>]
"""

import argparse
import os
import re
import shutil
import sys

MODEL_SIDES = {"measure-head": "head", "measure-base": "base"}
WHOLE = {"measure-verdict": "verdict", "compare-scorer-diff": "compare", "measure-timing": "timing"}
OPEN_FOLDERS = ("core-interior", "einhoven")
OPEN_PREFIXES = ("re1-", "nordic-", "r2025-", "sample-", "family-")
LEFT_OUT_PREFIXES = ("holdout-", "racadv-", "bimcollab-")
PROBE_OUTPUT = re.compile(r"probe(-pair)?(-.+)?\.txt")
ANSI = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")
LOG_TIME = re.compile(r"(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?Z) ?(.*)")
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


def drop_held_out_rows(path, held, notes):
    """Drop, from a Markdown table, the rows whose first cell is a file a held-out list names."""
    with open(path, encoding="utf-8", errors="replace") as handle:
        lines = handle.read().splitlines(keepends=True)
    kept = [line for line in lines if not (line.startswith("|") and line.split("|")[1].strip() in held)]
    if len(kept) != len(lines):
        with open(path, "w", encoding="utf-8") as handle:
            handle.writelines(kept)
        notes.add(f"the {len(lines) - len(kept)} rows of `{os.path.basename(path)}` for held-out models")


def left_out_names(held):
    """The folder prefixes and names left out: the three kinds of model kept to summary counts, and
    a `spent-` pair a held-out list names."""
    return list(LEFT_OUT_PREFIXES) + [f"spent-{name}" for name in sorted(held)]


def drop_left_out_sections(path, held, notes):
    """Drop, from a `diff -r` listing, the sections whose paths are a left-out model's folder."""
    with open(path, encoding="utf-8", errors="replace") as handle:
        lines = handle.read().splitlines(keepends=True)
    out, keep, dropped = [], True, 0
    folder = re.compile(r"(^|[\s/])(" + "|".join(re.escape(p) for p in left_out_names(held)) + r")")
    for line in lines:
        if line.startswith(("diff ", "Only in ")):
            keep = not folder.search(line)
            dropped += not keep
        if keep:
            out.append(line)
    if dropped:
        with open(path, "w", encoding="utf-8") as handle:
            handle.writelines(out)
        notes.add(f"the {dropped} section{'' if dropped == 1 else 's'} of `{os.path.basename(path)}` that diff a left-out folder")


def kept_file(name, part):
    if part == "holdout":
        return name == "counts.txt"
    if PROBE_OUTPUT.fullmatch(name):
        return part == "probes"
    if part == "probes":
        return False
    return (name.endswith(".txt") or name in ("ifc.sha256", "diagnostics.json", "timings.log")
            or (name.startswith("witness-") and name.endswith(".json")))


def keep_models(source, out, spent, held, notes, part):
    """Copy one side's kept files; return (models kept, files kept)."""
    models = files = 0
    left_out_spent = [0]
    for top in sorted(os.listdir(source)):
        path = os.path.join(source, top)
        if not os.path.isdir(path):
            # The runner's own files (`runner-*.txt`, `vmstat-*.log`, `score-cache-*.log`) are not kept.
            if top.endswith(".txt") and not top.startswith("runner-") and (part == "outputs" or top == "commit.txt"):
                os.makedirs(out, exist_ok=True)
                shutil.copyfile(path, os.path.join(out, top))
                files += 1
            continue
        if part == "holdout" and top.startswith("holdout-"):
            pass
        elif top.startswith(LEFT_OUT_PREFIXES):
            notes.add(f"the {top.split('-', 1)[0]} models' folders, whose outputs Measure keeps to summary counts")
            continue
        elif part == "holdout":
            # The other folders' outputs are the run's first kept commit's.
            if not (top in OPEN_FOLDERS or top.startswith(OPEN_PREFIXES) or top.startswith("spent-")):
                sys.exit(f"{source}: a folder no rule here knows, {top}: say whether its model is one anyone can fetch first")
            continue
        if part == "holdout":
            # A held-out folder, let through above: only its `counts.txt` is kept.
            pass
        elif top.startswith("spent-"):
            name = top[len("spent-"):]
            if name not in spent or name in held:
                # Counted, not named: the name of a held-out model is what the scan below refuses.
                left_out_spent[0] += 1
                continue
        elif top not in OPEN_FOLDERS and not top.startswith(OPEN_PREFIXES):
            sys.exit(f"{source}: a folder no rule here knows, {top}: say whether its model is one anyone can fetch first")
        held_any = False
        for directory, _, names in os.walk(path):
            for name in sorted(names):
                if not kept_file(name, part):
                    continue
                rel = os.path.relpath(os.path.join(directory, name), source)
                target = os.path.join(out, rel)
                os.makedirs(os.path.dirname(target), exist_ok=True)
                shutil.copyfile(os.path.join(directory, name), target)
                files += 1
                held_any = True
        models += held_any
    if left_out_spent[0]:
        notes.add(f"{left_out_spent[0]} `spent-*` folder{'' if left_out_spent[0] == 1 else 's'} that no list names as spent, or that a held-out list names")
    return models, files


def keep_log(source, out, pattern):
    """Copy the lines of `source`/run.log that `pattern` finds; return (lines, jobs)."""
    path = os.path.join(source, "run.log")
    if not os.path.isfile(path):
        sys.exit(f"{path}: no log to read")
    found, jobs = [], []
    with open(path, encoding="utf-8", errors="replace") as handle:
        for raw in handle:
            cells = raw.rstrip("\n").split("\t", 2)
            if len(cells) != 3:
                continue
            job, _, rest = cells
            m = LOG_TIME.fullmatch(rest)
            time, text = (m.group(1), m.group(2)) if m else ("-", rest)
            text = ANSI.sub("", text)
            if pattern.search(text):
                found.append(f"{job}\t{time}\t{text}\n")
                if job not in jobs:
                    jobs.append(job)
    if not found:
        sys.exit(f"{path}: `{pattern.pattern}` finds no line")
    os.makedirs(os.path.join(out, "log"), exist_ok=True)
    with open(os.path.join(out, "log", "lines.txt"), "w", encoding="utf-8") as handle:
        handle.writelines(found)
    return len(found), jobs


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("source")
    parser.add_argument("out")
    parser.add_argument("--run", required=True)
    parser.add_argument("--part", choices=("outputs", "probes", "holdout", "log"), default="outputs")
    parser.add_argument("--lines", help="with --part log: a Python regular expression; the log lines it finds are kept")
    parser.add_argument("--root", default=os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9]+", args.run):
        sys.exit("--run must be a run id")
    if os.path.exists(args.out) and os.listdir(args.out):
        sys.exit(f"{args.out} is not empty")
    if (args.part == "log") != (args.lines is not None):
        sys.exit("--lines goes with --part log, and --part log needs --lines")
    os.makedirs(args.out, exist_ok=True)
    spent, held = spent_stems(args.root)
    if args.part == "log":
        try:
            pattern = re.compile(args.lines)
        except re.error as error:
            sys.exit(f"--lines is not a regular expression: {error}")
        count, jobs = keep_log(args.source, args.out, pattern)
        lines = [f"- `log/lines.txt`: {count} line{'' if count == 1 else 's'} from {', '.join(f'`{j}`' for j in jobs)}"]
        with open(os.path.join(args.out, "KEPT.md"), "w", encoding="utf-8") as handle:
            handle.write(
                f"# Run {args.run}: kept log lines\n\n"
                f"From the log of https://github.com/DrunkOnJava/rvt-rs/actions/runs/{args.run}, by "
                "`tools/ci/keep-run-outputs.py` (#2124, #2148), which says what is kept and why: the lines "
                f"in which the pattern `{args.lines}` is found.\n\n" + "\n".join(lines) + "\n\n"
                "Left out: every other line of the log.\n"
            )
        return scan(args.out, held, args.part, lines)
    notes, lines = set(), []
    for artifact, side in MODEL_SIDES.items():
        path = os.path.join(args.source, artifact)
        if os.path.isdir(path):
            models, files = keep_models(path, os.path.join(args.out, side), spent, held, notes, args.part)
            lines.append(f"- `{side}/`, from `{artifact}`: {files} file{'' if files == 1 else 's'} for {models} model{'' if models == 1 else 's'}")
    for artifact, side in WHOLE.items():
        path = os.path.join(args.source, artifact)
        if os.path.isdir(path) and args.part == "outputs":
            shutil.copytree(path, os.path.join(args.out, side))
            for directory, _, names in os.walk(os.path.join(args.out, side)):
                for name in names:
                    if name.endswith(".txt"):
                        drop_left_out_sections(os.path.join(directory, name), held, notes)
                    if name.endswith(".md"):
                        drop_held_out_rows(os.path.join(directory, name), held, notes)
            count = sum(len(f) for _, _, f in os.walk(os.path.join(args.out, side)))
            lines.append(f"- `{side}/`, from `{artifact}`: {count} file{'' if count == 1 else 's'}")
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
            f"# Measure run {args.run}: kept {({'outputs': 'outputs', 'probes': 'probe outputs', 'holdout': 'held-out counts'})[args.part]}\n\n"
            f"From the artifacts of https://github.com/DrunkOnJava/rvt-rs/actions/runs/{args.run}"
            + (f", whose head side measured `{commit}`" if commit else "")
            + ", by `tools/ci/keep-run-outputs.py` (#2124), which says what is kept and why:\n\n"
            + "\n".join(lines) + "\n\n"
            + {"outputs": "Left out: every log but `timings.log`, the runner's own files, the exports, `info.json`, MCP dumps "
                           "and the probes' own output",
                "probes": f"Left out: everything but the probes' own output and `commit.txt`, which `refs/evidence/{args.run}` keeps",
                "holdout": "Left out: everything but each held-out model's `counts.txt`, the summary counts Measure keeps of it "
                           "(Griffin, 2026-11-22), and `commit.txt`; the held-out folders' other files; and every other "
                           f"folder, whose outputs `refs/evidence/{args.run}` keeps"}[args.part]
            + "".join(f"; {note}" for note in sorted(notes)) + ".\n"
        )
    return scan(args.out, held, args.part, lines)


def scan(out, held, part, lines):
    """Print what was kept; fail on a PII-shaped pattern or a left-out folder's name in any file."""
    patterns = [re.compile(p) for p in PII]
    # The held-out part keeps held-out folders, so their names are no leak there; the other
    # left-out kinds stay refused.
    refused = [p for p in LEFT_OUT_PREFIXES if not (part == "holdout" and p == "holdout-")]
    patterns.append(re.compile(r"(^|[^A-Za-z0-9])(" + "|".join(re.escape(p) for p in refused) + r")[A-Za-z0-9]"))
    if part != "holdout":
        patterns += [re.compile(r"(^|[^A-Za-z0-9])" + re.escape(f"spent-{name}") + r"($|[^A-Za-z0-9_.-])") for name in sorted(held)]
    hits = []
    for directory, _, names in os.walk(out):
        for name in names:
            with open(os.path.join(directory, name), encoding="utf-8", errors="replace") as handle:
                text = handle.read()
            single = text.replace("\\\\", "\\")
            hits += [(os.path.relpath(os.path.join(directory, name), out), p.pattern)
                     for p in patterns if p.search(text) or p.search(single)]
    print("\n".join(lines))
    if hits:
        for path, pattern in hits:
            print(f"a PII-shaped pattern or a left-out folder's name, {pattern}, in {path}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())

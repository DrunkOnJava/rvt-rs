"""The verdict a Measure run's reader looks for first: which exports moved.

Measure's compare job lists every model as identical or differing, and its scorer
diff leaves out `ifc.sha256` and `diagnostics.json`. This prints, above that list:

- how many models' IFC (`ifc.sha256`) and diagnostics (`diagnostics.json`) equal
  the base's, naming the ones that differ or have no base;
- whether the scorer diff holds anything beyond what the head side alone writes
  (each model's `probe.txt`, the `probe: exit N` line it adds to `counts.txt`, and
  `areas_across_saves*.txt`), with the first lines that do;
- with a probe, the most common lines the probe printed across the models, and
  the models whose probe did not exit 0.

It judges nothing against a pull request's registered rows; those stay the
reader's. A directory or file that is missing is said to be missing.

Usage: python3 -I measure_verdict.py <measure-head> [<measure-base> [<scorer-diff.txt>]]
"""
import collections
import os
import re
import sys

# A summary over GitHub's 1 MiB fails the compare job: bound what this adds.
MOST_NAMED = 40
MOST_LINES = 25
HEAD_ONLY = re.compile(r"(probe\.txt|areas_across_saves[^ /]*\.txt)$")
PROBE_EXIT = re.compile(r"^\+?probe: exit (\d+)$")


def models(root):
    """Model directory names under `root`, or None where it is absent."""
    if not root or not os.path.isdir(root):
        return None
    return sorted(d for d in os.listdir(root) if os.path.isdir(os.path.join(root, d)))


def read(path):
    try:
        with open(path, "rb") as f:
            return f.read()
    except OSError:
        return None


def named(names):
    shown = ", ".join(names[:MOST_NAMED])
    return shown + (f", and {len(names) - MOST_NAMED} more" if len(names) > MOST_NAMED else "")


def compare(head, base, name, label):
    same, differ, no_base, held = 0, [], [], 0
    for model in models(head):
        mine = read(os.path.join(head, model, name))
        if mine is None:
            continue
        held += 1
        theirs = read(os.path.join(base, model, name))
        if theirs is None:
            no_base.append(model)
        elif theirs == mine:
            same += 1
        else:
            differ.append(model)
    if held == 0:
        print(f"- {label}: none kept by this run")
        return
    line = f"- {label}: **{same} of {held} identical**"
    if differ:
        line += f"; **{len(differ)} differ**: {named(differ)}"
    if no_base:
        line += f"; {len(no_base)} with no base: {named(no_base)}"
    print(line)


def scorer_residue(path):
    try:
        lines = open(path, errors="replace").read().splitlines()
    except OSError:
        print("- Scorer diff: none written")
        return
    residue, files, current = [], set(), None
    for line in lines:
        if line.startswith("Only in "):
            if not HEAD_ONLY.search(line):
                residue.append(line)
                files.add(line)
        elif line.startswith("diff -r"):
            current = line.rsplit(" ", 1)[-1]
        elif line.startswith(("---", "+++", "@@")):
            continue
        elif line.startswith(("+", "-")):
            if current and current.endswith("counts.txt") and PROBE_EXIT.match(line):
                continue
            residue.append(f"{current}: {line}")
            files.add(current)
    if not residue:
        print("- Scorer differences: **none** beyond what only the head side writes (probe output, areas across saves)")
        return
    print(f"- Scorer differences: **{len(residue)} lines in {len(files)} files** beyond what only the head side writes; the first:")
    print()
    print("```diff")
    for line in residue[:MOST_LINES]:
        print(line[:300])
    print("```")


def probes(head):
    counts, ran, failed = collections.Counter(), 0, []
    for model in models(head):
        text = read(os.path.join(head, model, "probe.txt"))
        status = read(os.path.join(head, model, "counts.txt"))
        exit_code = None
        for line in (status or b"").decode(errors="replace").splitlines():
            match = PROBE_EXIT.match(line)
            if match:
                exit_code = int(match.group(1))
        if exit_code not in (None, 0):
            failed.append(f"{model} (exit {exit_code})")
        if text is None:
            continue
        ran += 1
        counts.update(l for l in text.decode(errors="replace").splitlines() if l.strip())
    if not ran and not failed:
        return
    print(f"- Probe: output from {ran} models" + (f"; **did not exit 0**: {named(failed)}" if failed else ""))
    if counts:
        print()
        print(f"  The {min(MOST_LINES, len(counts))} most common of its {len(counts)} distinct lines, with the number of models printing each:")
        print()
        print("```")
        for line, n in counts.most_common(MOST_LINES):
            print(f"{n:5d}  {line[:200]}")
        print("```")


def main(argv):
    if not 2 <= len(argv) <= 4:
        sys.exit(__doc__)
    head = argv[1]
    base = argv[2] if len(argv) > 2 else None
    diff = argv[3] if len(argv) > 3 else None
    print("### Verdict")
    print()
    if models(head) is None:
        print(f"No head results ({head}).")
        return
    if models(base) is None:
        print("- No base side: nothing to compare.")
    else:
        compare(head, base, "ifc.sha256", "Exports (`ifc.sha256`)")
        compare(head, base, "diagnostics.json", "Diagnostics (`diagnostics.json`)")
        if diff:
            scorer_residue(diff)
    probes(head)
    print()


if __name__ == "__main__":
    main(sys.argv)

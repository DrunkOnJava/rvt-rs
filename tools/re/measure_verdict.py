"""The verdict a Measure run's reader looks for first: which exports moved.

Measure's compare job lists every model as identical or differing, and its scorer
diff leaves out `ifc.sha256` and `diagnostics.json`. This prints, above that list:

- how many models' IFC (`ifc.sha256`) and diagnostics (`diagnostics.json`) equal
  the base's, naming the ones that differ or have no base;
- whether the scorer diff holds anything beyond what the head side alone writes
  (each model's `probe.txt`, the `probe: exit N` line it adds to `counts.txt`, and
  `areas_across_saves*.txt`), with the first lines that do;
- with a probe, the most common lines the probe printed across the models, and
  the models whose probe did not exit 0;
- on each side, how many exports IfcOpenShell's validation and the EXPRESS rules
  read, which they skipped for size (named; the run otherwise says so only in
  each model's `schema.txt`) and how many they did not read at all (#2103).

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
# measure-reference-models.sh writes this in `schema.txt`; the held-out scrub keeps it in
# `counts.txt` with a `schema: ` prefix.
SKIPPED = re.compile(r"^(?:schema: )?validate: skipped, over (\d+) MB$", re.M)


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
    # Over both sides' models: a model the base wrote and the head did not is
    # an export that disappeared, the regression this line most needs to show.
    same, differ, no_base, no_head, held = 0, [], [], [], 0
    for model in sorted(set(models(head)) | set(models(base))):
        mine = read(os.path.join(head, model, name))
        theirs = read(os.path.join(base, model, name))
        if mine is None and theirs is None:
            continue
        held += 1
        if mine is None:
            no_head.append(model)
        elif theirs is None:
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
    if no_head:
        line += f"; **{len(no_head)} missing at head**: {named(no_head)}"
    if no_base:
        line += f"; {len(no_base)} with no base: {named(no_base)}"
    print(line)


def scorer_residue(path):
    try:
        lines = open(path, errors="replace").read().splitlines()
    except OSError:
        print("- Scorer diff: none written")
        return
    residue, files, current, headers = [], set(), None, 0
    for line in lines:
        if headers:
            # The `---` and `+++` lines right after `diff -r`; a content line
            # can start with `--` or `++` too, and is residue.
            headers -= 1
            if line.startswith(("---", "+++")):
                continue
        if line.startswith("Only in "):
            if not HEAD_ONLY.search(line):
                residue.append(line)
                files.add(line)
        elif line.startswith("Binary files "):
            residue.append(line)
            files.add(line)
        elif line.startswith("diff -r"):
            current = line.rsplit(" ", 1)[-1]
            headers = 2
        elif line.startswith("@@"):
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


VALIDATED = re.compile(r"^validate:\nfindings \d+$|^schema validate: findings \d+$", re.M)


def validation_state(root, model):
    """`validated`, `skipped` (with the limit in MB it was over) or `not validated`: a run with
    `scorers=none`, a check that did not run, or a model with no export writes no line."""
    for name in ("schema.txt", "counts.txt"):
        text = (read(os.path.join(root, model, name)) or b"").decode(errors="replace")
        match = SKIPPED.search(text)
        if match:
            return "skipped", int(match.group(1))
        if VALIDATED.search(text):
            return "validated", None
    return "not validated", None


def validation(head, base):
    """Say which exports were validated, which were skipped for size and which were not
    validated at all, on each side: a model a check never read must not read as one it
    passed (#2103)."""
    for side, root in (("head", head), ("base", base)):
        names = models(root)
        if names is None:
            continue
        validated, skipped, missing = 0, [], []
        for model in names:
            state, limit = validation_state(root, model)
            if state == "validated":
                validated += 1
            elif state == "skipped":
                skipped.append((model, limit))
            else:
                missing.append(model)
        line = f"- Validation (IfcOpenShell and the EXPRESS rules) at {side}: {validated} of {len(names)} exports validated"
        if skipped:
            over = " or ".join(f"{mb} MB" for mb in sorted({mb for _, mb in skipped}))
            line += f"; **{len(skipped)} skipped for size (over {over})**: {named([m for m, _ in skipped])}"
        if missing:
            line += f"; {len(missing)} not validated (the families, which are not validated, a run without the `schema` step, or no export): {named(missing)}"
        print(line)


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
    validation(head, base)
    probes(head)
    print()


if __name__ == "__main__":
    main(sys.argv)

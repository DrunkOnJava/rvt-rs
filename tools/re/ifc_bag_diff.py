#!/usr/bin/env python3
"""What two IFC exports of one model differ in, with the step numbering left out.

A line diff of two exports is useless once an entity is added early in a file:
every `#123` after it moves. Measure's semantic diff (tools/re/semantic_diff.py)
reads the MCP dump, which holds a sample of each class's property sets, so a
change to a few members of a class of ten thousand may not be in it. This reads
the two files whole: every entity line has its references (`#123`) replaced by
`#`, and the lines are counted as two bags. What only one file has is printed by
entity type, with examples.

Use it on the `model.ifc` of both sides of a Measure run dispatched with
`-f keep_ifc=true` (the files are in the `measure-base` and `measure-head`
result folders, under each model's name). The held-out, BIMcollab and
`racadv-*` models leave the runner as counts only, so a run keeps no IFC of
them.

    python3 -I tools/re/ifc_bag_diff.py BASE.ifc HEAD.ifc [EXAMPLES_PER_TYPE]

What it ignores: `IFCOWNERHISTORY` lines, which carry the time of the export.
What it cannot say: which entity a line belongs to when two entities have the
same text after the numbers are gone (an `IFCRELAGGREGATES` is told apart by its
GlobalId, and a property value is not: `IFCPROPERTYSINGLEVALUE('IsExternal',...)`
counted 79 times means 79 values, not which 79). Read the property set lines
(which carry the GlobalId of the set) for that.

Standard library only.
"""

import re
import sys
from collections import Counter

REFERENCE = re.compile(r"#\d+")
TYPE = re.compile(r"^#\s*=\s*([A-Z0-9_]+)")
IGNORED = {"IFCOWNERHISTORY"}


def entity_type(line):
    match = TYPE.match(line)
    return match.group(1) if match else "?"


def bag(path):
    counts = Counter()
    with open(path, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            if not line.startswith("#"):
                continue
            text = REFERENCE.sub("#", line.strip())
            if entity_type(text) in IGNORED:
                continue
            counts[text] += 1
    return counts


def report(title, side, examples):
    by_type = Counter()
    for line, count in side.items():
        by_type[entity_type(line)] += count
    print(f"\n{title}, by entity type:")
    for kind, count in by_type.most_common(12):
        print(f"  {count:7d}  {kind}")
    shown = Counter()
    print(f"{title}, examples:")
    for line, count in side.most_common():
        kind = entity_type(line)
        if shown[kind] >= examples:
            continue
        shown[kind] += 1
        print(f"  x{count} {line[:230]}")


def main():
    if len(sys.argv) not in (3, 4):
        sys.exit(__doc__)
    base_path, head_path = sys.argv[1], sys.argv[2]
    examples = int(sys.argv[3]) if len(sys.argv) == 4 else 3
    base, head = bag(base_path), bag(head_path)
    only_base, only_head = base - head, head - base
    print(f"base lines {sum(base.values())}, head lines {sum(head.values())}")
    print(
        f"only in base: {sum(only_base.values())} lines ({len(only_base)} distinct); "
        f"only in head: {sum(only_head.values())} lines ({len(only_head)} distinct)"
    )
    report("only in base", only_base, examples)
    report("only in head", only_head, examples)


main()

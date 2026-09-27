#!/usr/bin/env python3
"""Compare the GlobalIds of two IFC exports of the same model (#400).

usage: tools/ifc_globalid_stability.py BEFORE.ifc AFTER.ifc [--require-stable]

Export one .rvt with two rvt-rs builds (or before and after any change) and
compare. Products are matched by (entity, Tag, nth occurrence of that pair).
Reports, per entity type, how many matched products kept their GlobalId,
whole-file GlobalId uniqueness, and the share of all GlobalIds that survive.
With --require-stable it exits 1 when a matched product changed its GlobalId
or either file repeats one.
"""
import re
import sys
from collections import defaultdict, Counter

LINE = re.compile(r"^#(\d+)=\s*(IFC[A-Z0-9]+)\('([0-9A-Za-z_$]{22})',(#\d+,.*)\);\s*$")


def split_top(args):
    out, depth, cur, quote = [], 0, [], False
    for ch in args:
        if quote:
            cur.append(ch)
            if ch == "'":
                quote = False
            continue
        if ch == "'":
            quote = True
        elif ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
        elif ch == "," and depth == 0:
            out.append("".join(cur))
            cur = []
            continue
        cur.append(ch)
    out.append("".join(cur))
    return out


# Products whose 8th attribute (index 7 after GlobalId) is Tag.
def load(path):
    products = {}
    occurrences = Counter()
    gids = []
    for raw in open(path, encoding="utf-8", errors="replace"):
        m = LINE.match(raw.strip())
        if not m:
            continue
        _, entity, gid, rest = m.groups()
        gids.append(gid)
        attrs = split_top(rest)
        # rest starts at OwnerHistory; Tag is IfcElement attribute 8 (1-based)
        # -> rest index 6.
        if len(attrs) > 6 and attrs[4].startswith("#") and attrs[6].startswith("'"):
            tag = attrs[6].strip("'")
            key = (entity, tag)
            products[(entity, tag, occurrences[key])] = gid
            occurrences[key] += 1
    return products, gids


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    require_stable = "--require-stable" in sys.argv
    a_path, b_path = args[0], args[1]
    failures = 0
    a, a_gids = load(a_path)
    b, b_gids = load(b_path)
    for name, gids in (("before", a_gids), ("after", b_gids)):
        dup = [g for g, n in Counter(gids).items() if n > 1]
        print(f"{name}: {len(gids)} GlobalIds, {len(dup)} duplicated")
        failures += len(dup)
    per = defaultdict(lambda: [0, 0])
    for key in a.keys() & b.keys():
        per[key[0]][0] += 1
        if a[key] == b[key]:
            per[key[0]][1] += 1
    print(f"{'entity':32} {'matched':>8} {'kept id':>8} {'changed':>8}")
    total = [0, 0]
    for entity in sorted(per):
        m, k = per[entity]
        total[0] += m
        total[1] += k
        print(f"{entity:32} {m:8} {k:8} {m - k:8}")
    print(f"{'all tagged products':32} {total[0]:8} {total[1]:8} {total[0] - total[1]:8}")
    print(f"only before: {len(a.keys() - b.keys())}, only after: {len(b.keys() - a.keys())}")
    shared = len(set(a_gids) & set(b_gids))
    print(f"GlobalIds of before present in after: {shared}/{len(set(a_gids))}")
    failures += total[0] - total[1]
    if require_stable and failures:
        sys.exit(1)


if __name__ == "__main__":
    main()

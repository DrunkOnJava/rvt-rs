#!/usr/bin/env python3
"""What could set the first character of Revit 2015's element GlobalIds
(RE-208, #878).

On BIMcollab's Revit 2015 example, a stored IfcGUID string has the last 21
characters of an element's GlobalId in Revit's export, never the first. This
takes the probe output of `examples/probe_re208_episodes_2015.rs` (its `gid`
lines) and Revit's export, pairs each element with its stored string, and
reports how much each candidate tells about Revit's first character: the
conditional entropy H(first | stored first, candidate) against H(first |
stored first). Pure standard library.

Usage:

    python3 tools/re/episodes_first_char.py <probe.txt> <revit.ifc>
"""

import collections
import math
import re
import sys

ALPHABET = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$"
SKIP = {"IFCOPENINGELEMENT"}


def value(global_id):
    n = 0
    for ch in global_id:
        n = n * 64 + ALPHABET.index(ch)
    return n


def entropy(values):
    counts = collections.Counter(values)
    n = len(values)
    return -sum(c / n * math.log2(c / n) for c in counts.values())


def conditional(ys, xs):
    groups = collections.defaultdict(list)
    for x, y in zip(xs, ys):
        groups[x].append(y)
    return sum(len(g) / len(ys) * entropy(g) for g in groups.values())


def main():
    probe, reference = sys.argv[1], sys.argv[2]
    elements = {}
    for line in open(reference, encoding="utf-8", errors="replace"):
        m = re.match(r"#\d+=\s*(IFC\w+)\('([0-9A-Za-z_$]{22})'(.*)", line)
        if not m or m.group(1) in SKIP:
            continue
        tags = re.findall(r",'(\d+)'(?=[,)])", m.group(3))
        if tags:
            elements[int(tags[-1])] = (m.group(1), m.group(2))
    stored = collections.defaultdict(list)
    for line in open(probe, encoding="utf-8", errors="replace"):
        parts = line.split()
        if len(parts) >= 6 and parts[0] == "gid":
            stored[parts[4][1:]].append((parts[4], parts[5]))
    pairs = []
    for tag, (cls, gid) in sorted(elements.items()):
        for s, before in stored.get(gid[1:], []):
            pairs.append((tag, cls, gid, s, before))
    r = [ALPHABET.index(p[2][0]) for p in pairs]
    s = [ALPHABET.index(p[3][0]) for p in pairs]
    print(f"elements paired with a stored string: {len(pairs)}")
    print(f"H(Revit's first) {entropy(r):.3f}; H(Revit's first | stored first) {conditional(r, s):.3f}")
    shared = [value(p[2]) & ((1 << 126) - 1) for p in pairs]
    bits = sorted((conditional(r, list(zip(s, [(v >> i) & 1 for v in shared]))), i) for i in range(126))
    print("best single shared bit: " + ", ".join(f"bit {i} {h:.3f}" for h, i in bits[:3]))
    candidates = {
        "Tag mod 4": [p[0] % 4 for p in pairs],
        "Tag mod 3": [p[0] % 3 for p in pairs],
        "class": [p[1] for p in pairs],
        "the 24 bytes before the stored string": [p[4][:48] for p in pairs],
    }
    for name, xs in candidates.items():
        print(f"{name}: {len(set(xs))} values, H {conditional(r, list(zip(s, xs))):.3f}")
    # A candidate that sets the first character leaves one value in each of
    # its groups; show the largest groups' spread.
    print("Revit's first character in the largest groups of the 24 bytes before:")
    groups = collections.defaultdict(list)
    for p, ri in zip(pairs, r):
        groups[p[4][:48]].append(ri)
    for before, rs in sorted(groups.items(), key=lambda kv: -len(kv[1]))[:3]:
        print(f"  {len(rs)} strings: " + ", ".join(f"{k} {n}" for k, n in sorted(collections.Counter(rs).items())))


if __name__ == "__main__":
    main()

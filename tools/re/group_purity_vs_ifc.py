#!/usr/bin/env python3
"""How well each field's values, taken as groups, decide a boolean property
Revit's export writes (RE-241, #35, W7).

Reads a probe's `rec <tag> <record class> <field>=<value> ...` lines and
Revit's IFC export. For every boolean property Revit writes on those
elements (by Tag) and every field with at most 2,000 distinct values, it
gives each value the label most of its elements carry and counts how many
elements that gets right: 100% means the property is constant within each
of the field's groups (a field such as the host's ElementId then says the
host decides). It prints, per property, the 15 best fields with their
number of groups, and the groups of `ID.host` and `ID.type` that are mixed.

Usage:

    python3 tools/re/group_purity_vs_ifc.py <probe.txt> <revit.ifc>
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.util.element


def main():
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    rows = {}
    for line in open(args[0], encoding="utf-8", errors="replace"):
        parts = line.split()
        if len(parts) < 3 or parts[0] != "rec":
            continue
        rows[parts[1]] = dict(p.split("=", 1) for p in parts[3:] if "=" in p)
    f = ifcopenshell.open(args[1])
    truths = collections.defaultdict(dict)
    for element in f.by_type("IfcElement"):
        tag = str(element.Tag) if element.Tag else None
        if tag not in rows:
            continue
        for pset, props in ifcopenshell.util.element.get_psets(element, psets_only=True).items():
            for prop, value in props.items():
                if isinstance(value, bool):
                    truths[f"{pset}.{prop}"][tag] = value
    for prop, labels in sorted(truths.items()):
        fields = collections.defaultdict(lambda: collections.defaultdict(collections.Counter))
        for tag, label in labels.items():
            for field, value in rows[tag].items():
                fields[field][value][label] += 1
        scored = []
        for field, groups in fields.items():
            if len(groups) > 2000:
                continue
            right = sum(max(c.values()) for c in groups.values())
            covered = sum(sum(c.values()) for c in groups.values())
            scored.append((right, covered, len(groups), field))
        true_n = sum(labels.values())
        print(f"{prop}: {len(labels)} elements, true {true_n}, false {len(labels) - true_n}")
        for right, covered, n, field in sorted(scored, key=lambda s: (-s[0], s[2]))[:15]:
            print(f"  {right}/{covered}  {field}  ({n} groups)")
        for key in ("ID.host", "ID.type"):
            groups = fields.get(key, {})
            mixed = [(v, c) for v, c in groups.items() if len(c) > 1]
            print(f"  {key}: {len(groups)} groups, {len(mixed)} mixed")
            for v, c in sorted(mixed, key=lambda vc: -sum(vc[1].values()))[:10]:
                print(f"    {v}: true {c[True]}, false {c[False]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

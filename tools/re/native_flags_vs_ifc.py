#!/usr/bin/env python3
"""Score integer built-in parameters against Revit's `LoadBearing` and
`IsExternal` (RE-285, #1158).

Reads `probe_re285_flag_candidates`' `param <id> <class> <level> <parameter id>
<value>` lines and Revit's own IFC export, pairs the elements by `Tag` (the
ElementId), and prints counts only, in the `native readers:` lines
`tools/ci/scrub-holdout-results.sh` keeps for a model without a licence:
never an ElementId or a name.

For each (IFC class, flag) it prints how many elements Revit's export writes
the flag true and false on, and then every parameter that could be the flag:
one whose values are not all the same on the class and whose every value goes
with one state of the flag, as `value state count` triples.

Usage:

    python3 tools/re/native_flags_vs_ifc.py <probe output> <revit-export.ifc>

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.util.element

FLAGS = ("LoadBearing", "IsExternal")


def revit_flags(model):
    out = {}
    for element in model.by_type("IfcElement"):
        tag = getattr(element, "Tag", None)
        if not tag or not str(tag).isdigit():
            continue
        values = {}
        for pset in ifcopenshell.util.element.get_psets(element).values():
            for name in FLAGS:
                if isinstance(pset.get(name), bool):
                    values.setdefault(name, pset[name])
        out.setdefault(int(tag), (element.is_a(), values))
    return out


def read_params(path):
    params = collections.defaultdict(dict)
    for line in open(path):
        fields = line.split()
        if len(fields) != 6 or fields[0] != "param" or not fields[1].isdigit():
            continue
        try:
            value = int(fields[5])
            parameter = int(fields[4])
        except ValueError:
            continue
        params[int(fields[1])][(fields[3], parameter)] = value
    return params


def main():
    args = sys.argv[1:]
    if len(args) < 2:
        raise SystemExit(__doc__)
    params = read_params(args[0])
    exported = revit_flags(ifcopenshell.open(args[1]))
    for flag in FLAGS:
        classes = collections.defaultdict(list)
        for tag, (klass, values) in exported.items():
            if flag in values:
                classes[klass].append((tag, values[flag]))
        for klass, rows in sorted(classes.items()):
            true = sum(1 for _, v in rows if v)
            print(f"native readers: flag {klass} {flag} revit true {true} false {len(rows) - true}")
            holders = collections.defaultdict(lambda: collections.defaultdict(collections.Counter))
            have = collections.Counter()
            for tag, state in rows:
                held = params.get(tag, {})
                for key, value in held.items():
                    holders[key][value][state] += 1
                    have[key] += 1
            for key in sorted(holders):
                if have[key] != len(rows):
                    continue
                by_value = holders[key]
                if len(by_value) < 2:
                    continue
                if any(len(states) != 1 for states in by_value.values()):
                    continue
                if len({next(iter(states)) for states in by_value.values()}) < 2:
                    continue
                triples = "; ".join(
                    f"{value} {'true' if next(iter(states)) else 'false'} {sum(states.values())}"
                    for value, states in sorted(by_value.items())
                )
                print(
                    f"native readers: flag {klass} {flag} candidate {key[0]} parameter {key[1]} values ({triples})"
                )


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Score built-in parameters against Revit's `LoadBearing`, `IsExternal` and
`ThermalTransmittance` (RE-285, #1158).

Reads `probe_re285_flag_candidates`' `param <id> <class> <level> <parameter id>
<value>` and `paramd` (a double) lines and Revit's own IFC export, pairs the
elements by `Tag` (the ElementId), and prints counts only, in the
`native readers:` lines `tools/ci/scrub-holdout-results.sh` keeps for a model
without a licence: never an ElementId or a name.

For each (IFC class, flag) it prints how many elements Revit's export writes
the flag true and false on, and then every parameter that could be the flag:
one whose values are not all the same on the class and whose every value goes
with one state of the flag, as `value state count` triples. For each class
whose elements carry a `ThermalTransmittance` it prints every double
parameter held by all of them whose ratio to Revit's value is one number (to
1e-6), with the number: the unit factor between Revit's internal value and the
IFC's.

Usage:

    python3 tools/re/native_flags_vs_ifc.py <probe output> <revit-export.ifc>

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.util.element

FLAGS = ("LoadBearing", "IsExternal")
# Parameters printed with their values on every class, whether or not the
# flag takes both states there: FLOOR_PARAM_IS_STRUCTURAL, which a candidate
# scan found for floors, and FUNCTION_PARAM (RE-286).
WATCHED = (-1001954, -1001006)


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


def read_params(path, kind="param"):
    params = collections.defaultdict(dict)
    for line in open(path):
        fields = line.split()
        if len(fields) != 6 or fields[0] != kind or not fields[1].isdigit():
            continue
        try:
            value = float(fields[5]) if kind == "paramd" else int(fields[5])
            parameter = int(fields[4])
        except ValueError:
            continue
        params[int(fields[1])][(fields[3], parameter)] = value
    return params


def thermal(model):
    out = {}
    for element in model.by_type("IfcElement"):
        tag = getattr(element, "Tag", None)
        if not tag or not str(tag).isdigit():
            continue
        for pset in ifcopenshell.util.element.get_psets(element).values():
            value = pset.get("ThermalTransmittance")
            if isinstance(value, float):
                out.setdefault(int(tag), (element.is_a(), value))
    return out


def thermal_candidates(doubles, rows):
    by_class = collections.defaultdict(list)
    for tag, (klass, value) in rows.items():
        by_class[klass].append((tag, value))
    for klass, items in sorted(by_class.items()):
        print(f"native readers: thermal {klass} revit values {len(items)}, nonzero {sum(1 for _, v in items if v)}")
        distinct = len({round(value, 9) for _, value in items})
        print(f"native readers: thermal {klass} distinct values {distinct}")
        if distinct < 2:
            continue
        candidates = collections.defaultdict(list)
        for tag, value in items:
            for key, held in doubles.get(tag, {}).items():
                candidates[key].append((held, value))
        for key in sorted(candidates):
            pairs = candidates[key]
            if len(pairs) != len(items) or any(held == 0 for held, _ in pairs):
                continue
            ratios = [value / held for held, value in pairs]
            if max(ratios) - min(ratios) > 1e-6 * abs(max(ratios)) or max(ratios) == 0:
                continue
            print(
                f"native readers: thermal {klass} candidate {key[0]} parameter {key[1]} ratio {ratios[0]:.6f}"
            )


def read_named(path):
    named = collections.defaultdict(dict)
    for line in open(path):
        fields = line.split()
        if len(fields) == 6 and fields[0] == "named" and fields[1].isdigit():
            named[int(fields[1])][(fields[3], fields[4])] = fields[5]
    return named


def named_report(named, exported):
    for flag in FLAGS:
        cells = collections.defaultdict(collections.Counter)
        for tag, (klass, values) in exported.items():
            if flag not in values:
                continue
            for (level, name), value in named.get(tag, {}).items():
                if flag.lower() in name.lower() or (flag == "LoadBearing" and "bearing" in name.lower()):
                    cells[(klass, level, name, value)][values[flag]] += 1
        for key, states in sorted(cells.items()):
            counts = "; ".join(
                f"{'true' if state else 'false'} {count}" for state, count in sorted(states.items())
            )
            print(f"native readers: named {key[0]} {flag} parameter {key[1]} {key[2]} value {key[3]} ({counts})")


def main():
    args = sys.argv[1:]
    if len(args) < 2:
        raise SystemExit(__doc__)
    params = read_params(args[0])
    model = ifcopenshell.open(args[1])
    thermal_candidates(read_params(args[0], "paramd"), thermal(model))
    exported = revit_flags(model)
    named_report(read_named(args[0]), exported)
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
                if key[1] in WATCHED:
                    triples = "; ".join(
                        f"{value} {'true' if state else 'false'} {count}"
                        for value, states in sorted(holders[key].items())
                        for state, count in sorted(states.items())
                    )
                    print(
                        f"native readers: flag {klass} {flag} watched {key[0]} parameter {key[1]} held by {have[key]} of {len(rows)} values ({triples})"
                    )
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

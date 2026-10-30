#!/usr/bin/env python3
"""Score the scalar fields of walls', floors' and their types' native records
against Revit's `LoadBearing`, `IsExternal` and `ThermalTransmittance`
(RE-286, #1158).

Reads `probe_re286_native_fields`' `field <id> <record class> <object
class>#<n> <field> <value>` and `link <id> <type id>` lines and Revit's own
IFC export, pairs the elements by `Tag` (the ElementId), and prints counts
only, in the `native readers:` lines `tools/ci/scrub-holdout-results.sh` keeps
for a model without a licence: never an ElementId or a name.

An element's candidates are the fields of its own record and, through its
`link`, those of its type's record (`type` before the record class). For each
(IFC class, flag) with both states in Revit's export, every candidate held by
all the class's elements whose values each go with one state of the flag is
printed with its `value state count` triples. For a class whose elements carry
a `ThermalTransmittance` with at least two distinct values, every candidate
whose ratio to Revit's value is one number (to 1e-6) is printed with it. For a
flag with one state on a class, the candidates that are constant over the
class are not listed; only the flag's count is.

Usage:

    python3 tools/re/native_fields_vs_ifc.py <probe output> <revit-export.ifc>

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.util.element

FLAGS = ("LoadBearing", "IsExternal")


def tagged(model):
    for element in model.by_type("IfcElement"):
        tag = getattr(element, "Tag", None)
        if tag and str(tag).isdigit():
            yield int(tag), element


def revit_values(model):
    flags, thermal = {}, {}
    for tag, element in tagged(model):
        for pset in ifcopenshell.util.element.get_psets(element).values():
            for name in FLAGS:
                if isinstance(pset.get(name), bool):
                    flags.setdefault((tag, name), (element.is_a(), pset[name]))
            value = pset.get("ThermalTransmittance")
            if isinstance(value, float):
                thermal.setdefault(tag, (element.is_a(), value))
    return flags, thermal


def read_probe(path):
    fields = collections.defaultdict(dict)
    links = {}
    for line in open(path):
        parts = line.split()
        if len(parts) == 3 and parts[0] == "link" and parts[1].isdigit() and parts[2].isdigit():
            links[int(parts[1])] = int(parts[2])
        elif len(parts) == 6 and parts[0] == "field" and parts[1].isdigit():
            try:
                value = float(parts[5])
            except ValueError:
                continue
            fields[int(parts[1])][(parts[2], parts[3], parts[4])] = value
    return fields, links


def candidates_of(tag, fields, links):
    own = {("own",) + key: value for key, value in fields.get(tag, {}).items()}
    type_id = links.get(tag)
    if type_id is not None:
        own.update({("type",) + key: value for key, value in fields.get(type_id, {}).items()})
    return own


def text(value):
    return str(int(value)) if value == int(value) else f"{value:.6f}"


def main():
    args = sys.argv[1:]
    if len(args) < 2:
        raise SystemExit(__doc__)
    fields, links = read_probe(args[0])
    flags, thermal = revit_values(ifcopenshell.open(args[1]))
    for flag in FLAGS:
        classes = collections.defaultdict(list)
        for (tag, name), (klass, value) in flags.items():
            if name == flag:
                classes[klass].append((tag, value))
        for klass, rows in sorted(classes.items()):
            true = sum(1 for _, v in rows if v)
            print(f"native readers: field {klass} {flag} revit true {true} false {len(rows) - true}")
            if true in (0, len(rows)):
                continue
            held = collections.defaultdict(lambda: collections.defaultdict(collections.Counter))
            have = collections.Counter()
            for tag, state in rows:
                for key, value in candidates_of(tag, fields, links).items():
                    held[key][value][state] += 1
                    have[key] += 1
            for key in sorted(held):
                by_value = held[key]
                if have[key] != len(rows) or len(by_value) < 2:
                    continue
                if any(len(states) != 1 for states in by_value.values()):
                    continue
                if len({next(iter(states)) for states in by_value.values()}) < 2:
                    continue
                triples = "; ".join(
                    f"{text(value)} {'true' if next(iter(states)) else 'false'} {sum(states.values())}"
                    for value, states in sorted(by_value.items())
                )
                print(f"native readers: field {klass} {flag} candidate {' '.join(key)} values ({triples})")
    by_class = collections.defaultdict(list)
    for tag, (klass, value) in thermal.items():
        by_class[klass].append((tag, value))
    for klass, items in sorted(by_class.items()):
        distinct = len({round(value, 9) for _, value in items})
        print(f"native readers: field {klass} ThermalTransmittance revit values {len(items)}, distinct {distinct}")
        if distinct < 2:
            continue
        held = collections.defaultdict(list)
        for tag, value in items:
            for key, number in candidates_of(tag, fields, links).items():
                held[key].append((number, value))
        for key in sorted(held):
            pairs = held[key]
            if len(pairs) != len(items) or any(number == 0 for number, _ in pairs):
                continue
            ratios = [value / number for number, value in pairs]
            if max(ratios) == 0 or max(ratios) - min(ratios) > 1e-6 * abs(max(ratios)):
                continue
            print(
                f"native readers: field {klass} ThermalTransmittance candidate {' '.join(key)} ratio {ratios[0]:.6f}"
            )


if __name__ == "__main__":
    main()

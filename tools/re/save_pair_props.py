#!/usr/bin/env python3
"""Each property and quantity in two rvt-rs exports of the same project saved
by two Revit releases, element for element (RE-343).

Elements are paired by `Tag` (their ElementId, which a project keeps across
saves), and rooms, which have no `Tag`, by `GlobalId`. For each property set
and property name it prints how many paired elements give the same value
(floats to four places), a different one, or the value on one side only.
Only names where the two exports do not agree everywhere are printed.

As with `save_pair_bodies.py`, a 2026 or 2027 save is scored against the same
Autodesk sample saved by Revit 2024 or 2025.

Usage:

    python3 tools/re/save_pair_props.py <candidate.ifc> <reference.ifc>
"""

import argparse
import collections

import ifcopenshell
import ifcopenshell.util.element


def key_of(element):
    if element.is_a("IfcSpace"):
        return element.GlobalId
    return getattr(element, "Tag", None)


def properties(path):
    """Tag -> (class, {(set, name): value})."""
    model = ifcopenshell.open(path)
    out = {}
    for element in model.by_type("IfcProduct"):
        key = key_of(element)
        if not key or element.is_a("IfcOpeningElement"):
            continue
        values = {("", "Name"): element.Name}
        for qtos_only in (False, True):
            psets = ifcopenshell.util.element.get_psets(element, qtos_only=qtos_only)
            for pset, props in psets.items():
                for name, value in props.items():
                    if name == "id":
                        continue
                    if isinstance(value, float):
                        value = round(value, 4)
                    values[(pset, name)] = value
        out[key] = (element.is_a(), values)
    return out


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("candidate")
    parser.add_argument("reference")
    args = parser.parse_args()
    candidate = properties(args.candidate)
    reference = properties(args.reference)
    tally = collections.defaultdict(lambda: [0, 0, 0, 0])
    first = {}
    for key, (cls, values) in candidate.items():
        if key not in reference:
            continue
        theirs = reference[key][1]
        for name in set(values) | set(theirs):
            row = tally[name]
            if name in values and name in theirs:
                if values[name] == theirs[name]:
                    row[0] += 1
                else:
                    row[1] += 1
                    first.setdefault(name, f" ({cls} {key}: {values[name]!r}, reference {theirs[name]!r})")
            elif name in values:
                row[2] += 1
            else:
                row[3] += 1
    for (pset, name), (agree, differ, mine, theirs) in sorted(tally.items()):
        if differ or mine or theirs:
            print(
                f"{pset}.{name}: same {agree}, differ {differ}, "
                f"only candidate {mine}, only reference {theirs}{first.get((pset, name), '')}"
            )


if __name__ == "__main__":
    main()

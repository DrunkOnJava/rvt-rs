#!/usr/bin/env python3
"""Which text parameter of a room is a text property Revit's export writes on
its space (RE-234, #35, W7).

Reads `examples/probe_re234_room_text_parameters.rs`'s output and Revit's IFC
export. Spaces are matched by Name, which Revit's export gives the room
number. For each text property (label, text or identifier) of each property
set Revit writes on the spaces the probe lists, it counts, for each
parameter, how many of those spaces have it and how many have Revit's value,
and prints the three equal most often.

Usage:

    python3 tools/re/text_params_vs_psets.py <probe.txt> <revit.ifc> [--list]

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import collections
import sys

import ifcopenshell

TEXT = {"IfcLabel", "IfcText", "IfcIdentifier"}


def read_probe(path):
    rows = {}
    for line in open(path, encoding="utf-8", errors="replace"):
        parts = line.split()
        if len(parts) < 3 or parts[0] != "txt":
            continue
        values = {}
        for part in parts[3:]:
            key, _, value = part.partition("=")
            values[key] = value.replace("_", " ")
        rows[parts[1].replace("_", " ")] = values
    return rows


def revit_texts(path):
    out = collections.defaultdict(dict)
    for space in ifcopenshell.open(path).by_type("IfcSpace"):
        for rel in space.IsDefinedBy or ():
            if not rel.is_a("IfcRelDefinesByProperties"):
                continue
            pset = rel.RelatingPropertyDefinition
            if not pset.is_a("IfcPropertySet"):
                continue
            for prop in pset.HasProperties or ():
                if prop.is_a("IfcPropertySingleValue") and prop.NominalValue is not None:
                    if prop.NominalValue.is_a() in TEXT:
                        out[space.Name][f"{pset.Name}.{prop.Name}"] = str(
                            prop.NominalValue.wrappedValue
                        )
    return out


def main():
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    rows = read_probe(args[0])
    revit = revit_texts(args[1])
    tally = collections.defaultdict(lambda: collections.defaultdict(collections.Counter))
    totals = collections.Counter()
    for number, values in rows.items():
        for prop, value in revit.get(number, {}).items():
            totals[prop] += 1
            for param, text in values.items():
                counts = tally[prop][param]
                counts["n"] += 1
                if text == value:
                    counts["equal"] += 1
    print(f"probe: {len(rows)} rooms; Revit's export: {len(revit)} spaces with text properties")
    for prop in sorted(totals, key=lambda p: (-totals[p], p)):
        ranked = sorted(tally[prop].items(), key=lambda kv: (-kv[1]["equal"], kv[0]))[:3]
        text = "  ".join(f"{p} {c['n']}/{c['equal']}" for p, c in ranked if c["equal"])
        print(f"  {prop:<55} {totals[prop]:>5}  {text or '-'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

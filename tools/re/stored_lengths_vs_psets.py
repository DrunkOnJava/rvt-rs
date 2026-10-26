#!/usr/bin/env python3
"""Which stored length of an element or of its type is a length property
Revit's export writes (RE-231, #35, W7).

Reads `examples/probe_re231_stored_lengths.rs`'s or
`examples/probe_re231_record_lengths.rs`'s output and Revit's IFC
export. For each IFC class and property of a length or count measure, or
length base quantity (a
count compared as it is), over the
elements of that class the probe lists and Revit's export gives the
property, it counts, for each stored parameter, how many of those elements
store it and how many store Revit's value within 0.001 ft, and prints the
three parameters equal most often. A property on the element's type object
counts as the element's where the element has none of that name.

Usage:

    python3 tools/re/stored_lengths_vs_psets.py <probe.txt> <revit.ifc> [--list]

`--list` also prints, for each property, up to five elements no stored
length gives the value of. Needs IfcOpenShell (the version
tools/ci/requirements-ifc.txt pins).
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.util.unit

FEET = 0.3048
TOLERANCE_FEET = 1e-3
LENGTH_MEASURES = {
    "IfcLengthMeasure",
    "IfcPositiveLengthMeasure",
    "IfcNonNegativeLengthMeasure",
}
# Counts are compared as they are, with no unit.
COUNT_MEASURES = {"IfcCountMeasure", "IfcInteger"}


def read_probe(path):
    rows = {}
    for line in open(path, encoding="utf-8", errors="replace"):
        parts = line.split()
        if len(parts) < 4 or parts[0] != "len":
            continue
        lengths = {}
        for part in parts[4:]:
            key, _, value = part.rpartition("=")
            try:
                lengths[key] = float(value)
            except ValueError:
                continue
        rows[parts[1]] = (parts[2], lengths)
    return rows


def length_properties(definitions, metres):
    out = {}
    for pset in definitions:
        if pset is not None and pset.is_a("IfcElementQuantity"):
            for q in pset.Quantities or ():
                if q.is_a("IfcQuantityLength") and q.LengthValue is not None:
                    out[f"{pset.Name}.{q.Name}"] = q.LengthValue * metres / FEET
            continue
        if pset is None or not pset.is_a("IfcPropertySet"):
            continue
        for prop in pset.HasProperties or ():
            if not prop.is_a("IfcPropertySingleValue") or prop.NominalValue is None:
                continue
            kind = prop.NominalValue.is_a()
            if kind in LENGTH_MEASURES:
                out[f"{pset.Name}.{prop.Name}"] = prop.NominalValue.wrappedValue * metres / FEET
            elif kind in COUNT_MEASURES:
                out[f"{pset.Name}.{prop.Name}"] = float(prop.NominalValue.wrappedValue)
    return out


def revit_lengths(path):
    f = ifcopenshell.open(path)
    metres = ifcopenshell.util.unit.calculate_unit_scale(f)
    out = {}
    for element in f.by_type("IfcElement"):
        tag = getattr(element, "Tag", None)
        if not tag or element.is_a("IfcOpeningElement") or tag in out:
            continue
        own = length_properties(
            (
                rel.RelatingPropertyDefinition
                for rel in element.IsDefinedBy or ()
                if rel.is_a("IfcRelDefinesByProperties")
            ),
            metres,
        )
        # IFC4 relates an element to its type through IsTypedBy, IFC2X3
        # through IsDefinedBy.
        type_rels = list(getattr(element, "IsTypedBy", None) or ()) + [
            rel for rel in element.IsDefinedBy or () if rel.is_a("IfcRelDefinesByType")
        ]
        typed = {}
        for rel in type_rels:
            typed.update(length_properties(rel.RelatingType.HasPropertySets or (), metres))
        for key, value in typed.items():
            own.setdefault(key, value)
        out[tag] = own
    return out


def main():
    listing = "--list" in sys.argv
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    rows = read_probe(args[0])
    revit = revit_lengths(args[1])
    tally = collections.defaultdict(lambda: collections.defaultdict(collections.Counter))
    totals = collections.Counter()
    unexplained = collections.defaultdict(list)
    for tag, (cls, lengths) in rows.items():
        for prop, value in revit.get(tag, {}).items():
            key = (cls, prop)
            totals[key] += 1
            hit = False
            for param, stored in lengths.items():
                counts = tally[key][param]
                counts["n"] += 1
                if abs(stored - value) <= TOLERANCE_FEET:
                    counts["equal"] += 1
                    hit = True
            if not hit:
                unexplained[key].append((tag, value))
    print(f"probe: {len(rows)} elements; Revit's export: {len(revit)} tagged elements")
    print("class, property, elements Revit gives it: the three parameters equal most often (storing/equal)")
    for key in sorted(totals, key=lambda k: (-totals[k], k)):
        ranked = sorted(
            tally[key].items(), key=lambda kv: (-kv[1]["equal"], -kv[1]["n"], kv[0])
        )[:3]
        text = "  ".join(f"{p} {c['n']}/{c['equal']}" for p, c in ranked if c["equal"])
        missed = len(unexplained[key])
        print(f"  {key[0]:<22} {key[1]:<55} {totals[key]:>5}  none {missed:>5}  {text or '-'}")
    if listing:
        for key in sorted(unexplained):
            for tag, value in unexplained[key][:5]:
                print(f"  none {key[0]} {key[1]} {tag} Revit {value:.4f}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

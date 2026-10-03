#!/usr/bin/env python3
"""Score the property sets in an rvt-rs IFC against Revit's own IFC export.

Research tool for #35 (parameters), `reports/element-framing/RE-153-room-finishes.md`
and `RE-154-pset-parameters.md`.

Every single-value, enumerated and list property of every property set
Revit's export relates to an object is keyed by the object (an element by its `Tag`, a space by its
`Name`, a storey by its `Name`, the building as `building`), the set's name
and the property's name. For each key Revit writes, the rvt-rs export either
has the same value (value type and value as written, numbers within a
relative 1e-5, lengths in metres from each file's own length unit), a
different one, or none. rvt-rs's own sets
(`RvtElementRecordGeometry` and other non-`Pset_` sets) are left out, as are
objects rvt-rs does not write at all.

It prints the totals, then a line per (set, property): how many values
Revit writes on objects rvt-rs also writes, how many are equal, how many
differ, and how many are missing.

Usage:

    python3 tools/re/psets_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc> [--list N]

Needs IfcOpenShell (tested with 0.8.5).
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.util.unit


def key_of(obj):
    if obj.is_a("IfcBuilding"):
        return "building"
    if obj.is_a("IfcSpace") or obj.is_a("IfcBuildingStorey"):
        return f"{obj.is_a()}:{obj.Name}"
    tag = getattr(obj, "Tag", None)
    return f"tag:{tag}" if tag else None


def value_of(value, metres_per_unit):
    if value is None:
        return None
    raw = value.wrappedValue
    if "LengthMeasure" in value.is_a():
        raw = raw * metres_per_unit
    if isinstance(raw, (tuple, list)):
        raw = tuple(raw)
    return (value.is_a(), raw)


def same(a, b):
    """Equal values: the same type, and numbers within a relative 1e-5 (a
    micrometre on a metre-long pipe, below the writers' rounding)."""
    if a is None or b is None or a == "missing" or b == "missing":
        return a == b
    (type_a, raw_a), (type_b, raw_b) = a, b
    if type_a != type_b:
        return False
    if isinstance(raw_a, float) and isinstance(raw_b, (int, float)):
        return abs(raw_a - raw_b) <= 1e-5 * max(1.0, abs(raw_a), abs(raw_b))
    return raw_a == raw_b


def properties(path):
    """(object key, set, property) -> value, and the keys of every object."""
    f = ifcopenshell.open(path)
    metres_per_unit = ifcopenshell.util.unit.calculate_unit_scale(f)
    out = {}
    objects = set()
    for product in f.by_type("IfcObject"):
        key = key_of(product)
        if key:
            objects.add(key)
    for rel in f.by_type("IfcRelDefinesByProperties"):
        pset = rel.RelatingPropertyDefinition
        if not pset.is_a("IfcPropertySet") or not (pset.Name or "").startswith("Pset_"):
            continue
        for obj in rel.RelatedObjects:
            key = key_of(obj)
            if not key:
                continue
            for prop in pset.HasProperties or ():
                if prop.is_a("IfcPropertySingleValue"):
                    out[(key, pset.Name, prop.Name)] = value_of(prop.NominalValue, metres_per_unit)
                elif prop.is_a("IfcPropertyEnumeratedValue"):
                    values = tuple(
                        value_of(v, metres_per_unit) for v in prop.EnumerationValues or ()
                    )
                    out[(key, pset.Name, prop.Name)] = ("enumerated", values)
                elif prop.is_a("IfcPropertyListValue"):
                    values = tuple(value_of(v, metres_per_unit) for v in prop.ListValues or ())
                    out[(key, pset.Name, prop.Name)] = ("list", values)
    return out, objects


def main(argv):
    args = argv[1:]
    listing = 0
    if "--list" in args:
        i = args.index("--list")
        listing = int(args[i + 1])
        del args[i : i + 2]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    (ours, our_objects), (theirs, _) = properties(args[0]), properties(args[1])
    rows = collections.defaultdict(lambda: [0, 0, 0, 0])
    worst = []
    for (key, pset, prop), value in sorted(theirs.items()):
        if key not in our_objects:
            continue
        row = rows[(pset, prop)]
        row[0] += 1
        mine = ours.get((key, pset, prop), "missing")
        if same(mine, value):
            row[1] += 1
        elif mine == "missing":
            row[3] += 1
        else:
            row[2] += 1
            if len(worst) < listing:
                worst.append(f"  {key} {pset}.{prop}: rvt-rs {mine}, Revit {value}")
    totals = [sum(r[i] for r in rows.values()) for i in range(4)]
    extra = sum(1 for k in ours if k not in theirs)
    print(
        f"Revit values on objects rvt-rs writes: {totals[0]}; equal: {totals[1]}; "
        f"different: {totals[2]}; missing: {totals[3]}; rvt-rs values Revit does not write: {extra}"
    )
    for (pset, prop), (n, equal, different, missing) in sorted(rows.items()):
        print(f"  {pset}.{prop}: {n}, equal {equal}, different {different}, missing {missing}")
    for line in worst:
        print(line)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

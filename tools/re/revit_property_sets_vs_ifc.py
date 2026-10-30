#!/usr/bin/env python3
"""Score `rvt-ifc --revit-property-sets` against Revit's export with "Export Revit property sets" (RE-210).

With that option Revit's exporter writes each element's parameters in one
set per parameter group, named for the group (`Dimensions`, `Identity Data`,
`Other`, ...), and its type's in such sets on its type object. This pairs
each element of rvt-rs's export with Revit's of the same Tag (openings,
which share their door's or window's Tag, left out; the products of one
Tag, such as the slabs Revit splits a floor into, taken together) and, separately for the
element's own sets and its type object's, counts each property rvt-rs writes
in a set that is not a standard one (`Pset_`, `Qto_`) or rvt-rs's own
(`Rvt`):

- equal: Revit writes the same property in the set of the same name, with
  the same value (a length, area, volume or angle after each file's own
  unit; a number within rvt-rs's rounding to six decimals, or a relative
  1e-6);
- other set: Revit writes the property with that value, in another set;
- differ: Revit writes it in that set with another value;
- not written: Revit writes no such property there.

It also counts, per set, the properties Revit writes that rvt-rs does not,
and, among those that differ, the ones whose value in Revit's export is
rvt-rs's divided by 304.8 squared: Revit 2025's export writes a window type's
sill height and its nested families' sizes so, a 3 ft sill as 0.0098 mm
(RE-210 §9).

Usage:
    rvt-ifc MODEL.rvt -o ours.ifc --revit-property-sets
    python3 tools/re/revit_property_sets_vs_ifc.py ours.ifc REVIT.ifc [--list]

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.util.unit

RELATIVE = 1e-6
# rvt-rs writes a property's number with six decimals, in SI units.
ROUNDING = 5e-7 * (1 + 1e-9)
POWERS = {
    "IfcLengthMeasure": 1,
    "IfcPositiveLengthMeasure": 1,
    "IfcNonNegativeLengthMeasure": 1,
    "IfcAreaMeasure": 2,
    "IfcVolumeMeasure": 3,
}


def own_set(name):
    return not (name.startswith("Pset_") or name.startswith("Qto_") or name.startswith("Rvt"))


def sets_of(path):
    """Tag -> {"instance"|"type": {(set, property): (value, IFC type)}}."""
    model = ifcopenshell.open(path)
    scale = ifcopenshell.util.unit.calculate_unit_scale(model)
    angle = ifcopenshell.util.unit.calculate_unit_scale(model, "PLANEANGLEUNIT")

    def values(psets):
        out = {}
        for pset in psets:
            if pset is None or not pset.is_a("IfcPropertySet") or not own_set(pset.Name or ""):
                continue
            for prop in pset.HasProperties or ():
                if prop.is_a("IfcPropertySingleValue") and prop.NominalValue is not None:
                    value, kind = prop.NominalValue.wrappedValue, prop.NominalValue.is_a()
                    if kind in POWERS and isinstance(value, (int, float)):
                        value = value * scale ** POWERS[kind]
                    elif kind == "IfcPlaneAngleMeasure" and isinstance(value, (int, float)):
                        value = value * angle
                    out[(pset.Name, prop.Name)] = (value, kind)
        return out

    # An element Revit writes as several products of one Tag (a floor split
    # into slabs) is compared with the sets of them all. Openings, which
    # share their door's or window's Tag, are left out.
    by_tag = {}
    for product in model.by_type("IfcElement"):
        tag = getattr(product, "Tag", None)
        if not tag or product.is_a("IfcOpeningElement"):
            continue
        own = values(
            rel.RelatingPropertyDefinition
            for rel in product.IsDefinedBy or ()
            if rel.is_a("IfcRelDefinesByProperties")
        )
        typed = {}
        for rel in getattr(product, "IsTypedBy", None) or ():
            typed.update(values(rel.RelatingType.HasPropertySets or ()))
        merged = by_tag.setdefault(tag, {"instance": {}, "type": {}})
        merged["instance"].update(own)
        merged["type"].update(typed)
    return by_tag


def same(a, b):
    (x, _), (y, _) = a, b
    if isinstance(x, bool) or isinstance(y, bool) or isinstance(x, str) or isinstance(y, str):
        return x == y
    return abs(x - y) <= max(ROUNDING, RELATIVE * max(abs(x), abs(y)))


def shrunk_by_feet(ours, theirs):
    a, b = ours[0], theirs[0]
    if not all(isinstance(v, (int, float)) and not isinstance(v, bool) for v in (a, b)):
        return False
    return b != 0 and same((a, None), (b * 304.8 * 304.8, None))


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if len(args) != 2:
        sys.exit(__doc__)
    ours, revit = sets_of(args[0]), sets_of(args[1])
    counts = collections.defaultdict(collections.Counter)
    missing = collections.Counter()
    shrunk = 0
    rows = []
    for tag, by_level in ours.items():
        theirs = revit.get(tag)
        if theirs is None:
            continue
        for level, values in by_level.items():
            revit_values = theirs[level]
            for key, value in values.items():
                set_name, prop = key
                if key in revit_values:
                    verdict = "equal" if same(value, revit_values[key]) else "differ"
                    if verdict == "differ" and shrunk_by_feet(value, revit_values[key]):
                        shrunk += 1
                elif any(p == prop and same(value, v) for (s, p), v in revit_values.items()):
                    verdict = "other set"
                else:
                    verdict = "not written"
                counts[(level, set_name)][verdict] += 1
                if verdict != "equal":
                    rows.append((tag, level, set_name, prop, value[0], revit_values.get(key, ("-",))[0]))
            for key in revit_values:
                if key not in values:
                    missing[(level, key[0])] += 1
    total = collections.Counter()
    print("| level | set | equal | other set | differ | not written | Revit's, not written by rvt-rs |")
    print("|---|---|---:|---:|---:|---:|---:|")
    for key in sorted(set(counts) | set(missing)):
        c = counts[key]
        total.update(c)
        print(f"| {key[0]} | {key[1]} | {c['equal']} | {c['other set']} | {c['differ']} | {c['not written']} | {missing[key]} |")
    print(
        f"\nwritten by rvt-rs: {sum(total.values())}; equal {total['equal']}, other set {total['other set']}, "
        f"differ {total['differ']}, not written by Revit {total['not written']}; "
        f"Revit's not written by rvt-rs: {sum(missing.values())}"
    )
    if shrunk:
        print(f"differ where Revit's value is rvt-rs's divided by 304.8 squared: {shrunk}")
    if "--list" in sys.argv:
        for row in rows:
            print("row", *row, sep="\t")


if __name__ == "__main__":
    main()

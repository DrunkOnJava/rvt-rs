#!/usr/bin/env python3
"""A wall's, floor's, ceiling's and roof's type parameters against Revit's
export with "Export Revit property sets" (RE-256, #35).

Reads `rvt-elements --no-typed` JSON and Revit's IFC export. For each
`IfcWall`, `IfcSlab`, `IfcCovering` and `IfcRoof` of Revit's export whose
Tag is an element rvt-rs lists, each property Revit writes in its own sets
(not `Pset_`, `Qto_`) on the element's type object is matched when one of
the element's type-level parameters equals it: a text, a Yes/No, a number
within 1e-6 relative, or an ElementId by its target's name. Prints, per
property, the elements matched of those carrying it and the parameter ids
that matched.

Usage:
    rvt-elements --no-typed MODEL.rvt > elements.json
    python3 tools/re/system_type_parameters_vs_ifc.py elements.json REVIT.ifc

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import collections
import json
import sys

import ifcopenshell
import ifcopenshell.util.element

CLASSES = ("IfcWall", "IfcSlab", "IfcCovering", "IfcRoof")


def same(ours, theirs):
    v = ours["value"]
    if isinstance(theirs, bool) or isinstance(v, bool):
        return isinstance(v, bool) and v == theirs
    if isinstance(theirs, str):
        return v == theirs or ours.get("target_name") == theirs
    if isinstance(theirs, (int, float)) and isinstance(v, (int, float)):
        return abs(v - theirs) <= 1e-6 * max(1.0, abs(theirs))
    return False


def main():
    elements = {e["id"]: e for e in json.load(open(sys.argv[1]))}
    model = ifcopenshell.open(sys.argv[2])
    totals = collections.defaultdict(collections.Counter)
    ids = collections.defaultdict(collections.Counter)
    listed = collections.Counter()
    typed = collections.Counter()
    for cls in CLASSES:
        for element in model.by_type(cls):
            try:
                tag = int(element.Tag)
            except (TypeError, ValueError):
                continue
            type_object = ifcopenshell.util.element.get_type(element)
            ours = elements.get(tag)
            if not type_object or not ours:
                continue
            listed[cls] += 1
            values = [p for p in ours.get("parameters", []) if p["level"] == "type"]
            if values:
                typed[cls] += 1
            for pset in type_object.HasPropertySets or ():
                if not pset.is_a("IfcPropertySet") or pset.Name.startswith(("Pset_", "Qto_")):
                    continue
                for prop in pset.HasProperties:
                    if not prop.is_a("IfcPropertySingleValue") or prop.NominalValue is None:
                        continue
                    key = f"{cls} {pset.Name.replace('(Type)', '')}.{prop.Name}"
                    hits = {p["parameter_id"] for p in values if same(p, prop.NominalValue.wrappedValue)}
                    totals[key]["matched" if hits else "unmatched"] += 1
                    totals[key]["n"] += 1
                    for hit in hits:
                        ids[key][hit] += 1
    print("typed in Revit's export and listed:", dict(listed))
    print("with type-level values:", dict(typed))
    for key, count in sorted(totals.items(), key=lambda kv: -kv[1]["n"]):
        print(f"{count['matched']:5}/{count['n']:<5} {key:62} {ids[key].most_common(3)}")


if __name__ == "__main__":
    main()

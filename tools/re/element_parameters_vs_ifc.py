#!/usr/bin/env python3
"""Score `rvt-elements`' saved parameters against Revit's export (RE-207).

`rvt-elements MODEL.rvt` lists, per element, the parameters its native record
and its type's hold (`src/research/native_element_parameters.rs`), each named by its
definition's caption. Revit's exporter writes a property `<set>.<property>`
from the element's parameter of that name before anything else (RE-194), so
such a parameter's value is the value Revit writes there. A Revit export made
with "Export Revit property sets" also writes every parameter under its own
name, in a set named for its group; the parameter of that name is then paired
the same way.

This pairs each element with Revit's of the same Tag (the ElementId), takes
every property Revit writes on it (its own sets, then its type object's), and
counts, per parameter name, how many of rvt-rs's values equal Revit's:

- a Yes/No (a boolean) or an integer against an IfcBoolean, IfcLogical,
  IfcInteger or count;
- a text against any string;
- a length, area or volume against the SI value rvt-rs gives, after the
  export's own length unit is applied; any other number as stored, within a
  relative 1e-6;
- an ElementId against the name of the material or family type it names.

A family parameter named `Fire Rating` is also paired with the `FireRating`
Revit writes in the element's common set, where the element has no parameter
named for that property (which Revit takes first): on the R2025 models (#356)
a window's `Pset_WindowCommon.FireRating` is its type's `Fire Rating`
(RE-185).

A parameter Revit writes nothing for on that element is counted apart, as not
written: Revit's export leaves some out (RE-194 section 4).

Usage:
    rvt-elements MODEL.rvt --no-typed > elements.json
    python3 tools/re/element_parameters_vs_ifc.py elements.json REVIT.ifc [--list]

The first argument may also be `probes/examples/probe_re207_element_parameters`'
output, one element per line, as the Measure workflow records it.

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import collections
import json
import sys

import ifcopenshell
import ifcopenshell.util.unit

RELATIVE = 1e-6
# A parameter name -> the property name Revit's export writes its value as.
ALIASES = {"Fire Rating": "FireRating"}
BOOLEAN = {"IfcBoolean", "IfcLogical"}
INTEGER = {"IfcInteger", "IfcCountMeasure"}
SCALED = {
    "IfcLengthMeasure": 1,
    "IfcPositiveLengthMeasure": 1,
    "IfcNonNegativeLengthMeasure": 1,
    "IfcAreaMeasure": 2,
    "IfcVolumeMeasure": 3,
}


def revit_values(path):
    """Tag -> {property name: (value, IFC type)} for each product, with the
    set-qualified `<set>.<property>` and the plain property name both keyed;
    the element's own sets win over its type's. Also the length unit scale."""
    model = ifcopenshell.open(path)
    scale = ifcopenshell.util.unit.calculate_unit_scale(model)

    def single_values(sets, out):
        for pset in sets:
            if pset is None or not pset.is_a("IfcPropertySet"):
                continue
            for prop in pset.HasProperties or ():
                if not prop.is_a("IfcPropertySingleValue") or prop.NominalValue is None:
                    continue
                value = (prop.NominalValue.wrappedValue, prop.NominalValue.is_a())
                out.setdefault(f"{pset.Name}.{prop.Name}", value)
                out.setdefault(prop.Name, value)

    by_tag = {}
    products = sorted(model.by_type("IfcProduct"), key=lambda p: p.is_a("IfcOpeningElement"))
    for product in products:
        tag = getattr(product, "Tag", None)
        if not tag:
            continue
        values = {}
        single_values(
            (
                rel.RelatingPropertyDefinition
                for rel in getattr(product, "IsDefinedBy", None) or ()
                if rel.is_a("IfcRelDefinesByProperties")
            ),
            values,
        )
        for rel in getattr(product, "IsTypedBy", None) or ():
            single_values(rel.RelatingType.HasPropertySets or (), values)
        # A door's or window's opening carries the door's or window's Tag.
        if product.is_a("IfcOpeningElement") and tag in by_tag:
            continue
        by_tag[tag] = values
    return by_tag, scale


def equal(parameter, revit, scale):
    value, ifc_type = revit
    ours = parameter.get("value")
    if parameter.get("storage_type") == "ElementId":
        return parameter.get("target_name") is not None and str(value) == parameter["target_name"]
    if ifc_type in BOOLEAN:
        if isinstance(ours, bool):
            return ours == (value is True or value == "T")
        return isinstance(ours, int) and (ours != 0) == (value is True or value == "T")
    if ifc_type in INTEGER:
        return isinstance(ours, int) and not isinstance(ours, bool) and ours == value
    if isinstance(value, str):
        return isinstance(ours, str) and ours == value
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        if ifc_type in SCALED:
            ours = parameter.get("si_value")
            value = value * scale ** SCALED[ifc_type]
        if not isinstance(ours, (int, float)) or isinstance(ours, bool):
            return False
        return abs(ours - value) <= RELATIVE * max(abs(ours), abs(value), 1e-12)
    return False


def load(path):
    """`rvt-elements`' JSON array, or the probe's output: one JSON object per
    element line, other lines (its counts, Measure's exit status) skipped."""
    with open(path, encoding="utf-8") as f:
        text = f.read()
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        pass
    elements = []
    for line in text.splitlines():
        try:
            element = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(element, dict) and "parameters" in element:
            elements.append(element)
    return elements


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if len(args) != 2:
        sys.exit(__doc__)
    listing = "--list" in sys.argv
    elements = load(args[0])
    by_tag, scale = revit_values(args[1])
    counts = collections.defaultdict(collections.Counter)
    differ = []
    for element in elements:
        revit = by_tag.get(str(element.get("id")))
        if revit is None:
            continue
        named = {p.get("name") or "" for p in element.get("parameters", ())}
        for parameter in element.get("parameters", ()):
            name = parameter.get("name")
            if "parameter_id" not in parameter or not name:
                continue
            key = (name, parameter.get("level"))
            alias = ALIASES.get(name)
            if alias and any(n == alias or n.endswith("." + alias) for n in named):
                continue
            if name not in revit and alias:
                name = alias
            if name not in revit:
                counts[key]["not written"] += 1
            elif equal(parameter, revit[name], scale):
                counts[key]["equal"] += 1
            else:
                counts[key]["differ"] += 1
                differ.append((element["id"], name, parameter.get("value"), revit[name]))
    total = collections.Counter()
    print("| parameter | level | equal | differ | not written |")
    print("|---|---|---:|---:|---:|")
    for (name, level), c in sorted(counts.items(), key=lambda kv: -sum(kv[1].values())):
        total.update(c)
        if c["equal"] or c["differ"]:
            print(f"| {name} | {level} | {c['equal']} | {c['differ']} | {c['not written']} |")
    print(
        f"\npaired: {total['equal'] + total['differ']}, equal {total['equal']}, "
        f"differ {total['differ']}; not written by Revit: {total['not written']}"
    )
    if listing:
        for row in differ:
            print("differ", *row, sep="\t")


if __name__ == "__main__":
    main()

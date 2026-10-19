#!/usr/bin/env python3
"""Score the parameters named `<property set>.<property>` against Revit's export (RE-194).

`examples/probe_re194_pset_parameters.rs` prints, per element, the entry of
each project or shared parameter whose name starts with `Pset_` in the
element's own data object, read as an integer (`int`), as text (`text`) and as
an `f64` after the key (`after`), before an `ff` x 8 run (`before`) or just
before the key (`just_before`). Revit's
exporter writes such a parameter as the property it names (`Pset_WallCommon.
FireRating` on a wall), before anything else (RE-193). This pairs each element
with Revit's of the same Tag, else GlobalId, and counts per parameter which
reading equals the value Revit writes in that set on that element: a Boolean
against `int` (0 is false), a string against `text`, a number against `after`,
`before` or `just_before` as the IFC type Revit writes it says: a length in
metres from feet and an area in square metres from square feet (the export's
own length unit applied), any other number as stored. Each value is placed on
the element itself ("occurrence") when a set the element carries holds that
property, else on its type.

Usage:
    python3 tools/re/pset_parameters_vs_ifc.py PROBE.txt REVIT.ifc [--list]

Needs IfcOpenShell (tested with 0.8.5).
"""

import collections
import json
import sys

import ifcopenshell
import ifcopenshell.util.element
import ifcopenshell.util.unit

FEET = 0.3048
RELATIVE = 1e-6


def revit_values(path):
    """(Tag -> {(set, property): (value, IFC type, entity, where)}, GlobalId -> the same),
    the unit scale. `where` is "occurrence" for a property a set on the element itself
    holds, else "type" for one its type object's sets hold."""
    model = ifcopenshell.open(path)
    scale = ifcopenshell.util.unit.calculate_unit_scale(model)

    def single_values(sets):
        out = {}
        for pset in sets:
            if pset is None or not pset.is_a("IfcPropertySet"):
                continue
            for prop in pset.HasProperties or ():
                if prop.is_a("IfcPropertySingleValue") and prop.NominalValue is not None:
                    out[(pset.Name, prop.Name)] = (prop.NominalValue.wrappedValue, prop.NominalValue.is_a())
        return out

    by_tag, by_global_id = {}, {}
    for product in model.by_type("IfcProduct"):
        own = single_values(
            rel.RelatingPropertyDefinition
            for rel in getattr(product, "IsDefinedBy", None) or ()
            if rel.is_a("IfcRelDefinesByProperties")
        )
        type_object = ifcopenshell.util.element.get_type(product)
        inherited = single_values(getattr(type_object, "HasPropertySets", None) or ()) if type_object else {}
        values = {key: (value, kind, product.is_a(), "type") for key, (value, kind) in inherited.items()}
        values.update({key: (value, kind, product.is_a(), "occurrence") for key, (value, kind) in own.items()})
        if not values:
            continue
        # A door's or window's opening carries the door's Tag; the door is the
        # element the parameter is on.
        element = product.is_a("IfcElement") and not product.is_a("IfcFeatureElementSubtraction")
        tag = getattr(product, "Tag", None) if element else None
        if tag:
            by_tag[str(tag)] = values
        by_global_id[product.GlobalId] = values
    return by_tag, by_global_id, scale


def close(a, b):
    return abs(a - b) <= RELATIVE * max(1.0, abs(a), abs(b))


def matches(revit, kind, reading, scale):
    """The name of the first reading equal to Revit's value, of IFC type `kind`, or None."""
    if isinstance(revit, bool):
        return "int" if reading.get("int") is not None and (reading["int"] != 0) == revit else None
    if isinstance(revit, str):
        return "text" if reading.get("text") == revit else None
    if isinstance(revit, (int, float)):
        label, factor = "", 1.0
        if "LengthMeasure" in kind:
            label, factor = " in metres", FEET / scale
        elif "AreaMeasure" in kind:
            label, factor = " in square metres", FEET * FEET / (scale * scale)
        for key in ("after", "before", "just_before"):
            value = reading.get(key)
            if value is not None and close(value * factor, float(revit)):
                return key + label
        if not label and reading.get("int") is not None and close(float(reading["int"]), float(revit)):
            return "int"
    return None


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    listing = "--list" in sys.argv
    probe, reference = args
    rows, parameters, conflicting = [], [], {}
    for line in open(probe, encoding="utf-8", errors="replace"):
        line = line.strip()
        if not line.startswith("{"):
            continue
        row = json.loads(line)
        if "parameters" in row:
            parameters = row["parameters"]
        elif "conflicting" in row:
            conflicting = row["conflicting"]
        elif row.get("id") is not None:
            rows.append(row)
    by_tag, by_global_id, scale = revit_values(reference)
    print(
        f"parameters named Pset_*: {len(parameters)}; elements carrying one: {len(rows)}; "
        f"entries left out, their latest copies disagreeing: {sum(conflicting.values())}"
    )
    tally = collections.defaultdict(collections.Counter)
    placement = collections.defaultdict(collections.Counter)
    examples = collections.defaultdict(list)
    for row in rows:
        revit = by_tag.get(str(row["id"])) or by_global_id.get(row.get("global_id") or "")
        for name, reading in row["values"].items():
            set_name, _, prop = name.partition(".")
            if revit is None:
                tally[name]["element not in Revit's export"] += 1
                continue
            if (set_name, prop) not in revit:
                tally[name]["Revit writes no such property on it"] += 1
                continue
            value, kind, entity, where = revit[(set_name, prop)]
            how = matches(value, kind, reading, scale)
            if how is None:
                tally[name]["different"] += 1
                if len(examples[name]) < 3:
                    examples[name].append((row["id"], row["class"], value, reading))
            else:
                tally[name][f"equal as {how}"] += 1
                placement[name][(entity, where)] += 1
    for name in sorted(tally):
        counts = ", ".join(f"{label} {n}" for label, n in sorted(tally[name].items()))
        where = ", ".join(f"{entity} {at} {n}" for (entity, at), n in sorted(placement[name].items()))
        print(f"  {name}: {counts}" + (f" [Revit writes the equal ones on: {where}]" if where else ""))
        if listing:
            for element, cls, value, reading in examples[name]:
                print(f"      {element} {cls}: Revit {value!r}; read {reading}")


main()

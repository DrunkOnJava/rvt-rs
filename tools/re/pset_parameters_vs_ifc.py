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
against `int` (0 is false), a string against `text`, a number against `after`
or `before`, as stored, in metres from feet, or in square metres from square
feet (the export's own length unit applied).

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
    """(Tag -> {(set, property): (value, entity, where)}, GlobalId -> the same), the unit
    scale. `where` is "occurrence" for a set on the element itself and "type" for one
    its type object carries."""
    model = ifcopenshell.open(path)
    scale = ifcopenshell.util.unit.calculate_unit_scale(model)
    by_tag, by_global_id = {}, {}
    for product in model.by_type("IfcProduct"):
        values = {}
        own = ifcopenshell.util.element.get_psets(product, psets_only=True, should_inherit=False)
        for set_name, properties in ifcopenshell.util.element.get_psets(product, psets_only=True).items():
            where = "occurrence" if set_name in own else "type"
            for key, value in properties.items():
                if key != "id" and value is not None:
                    values[(set_name, key)] = (value, product.is_a(), where)
        if not values:
            continue
        tag = getattr(product, "Tag", None) if product.is_a("IfcElement") else None
        if tag:
            by_tag[str(tag)] = values
        by_global_id[product.GlobalId] = values
    return by_tag, by_global_id, scale


def close(a, b):
    return abs(a - b) <= RELATIVE * max(1.0, abs(a), abs(b))


def matches(revit, reading, scale):
    """The name of the first reading equal to Revit's value, or None."""
    if isinstance(revit, bool):
        return "int" if reading.get("int") is not None and (reading["int"] != 0) == revit else None
    if isinstance(revit, str):
        return "text" if reading.get("text") == revit else None
    if isinstance(revit, (int, float)):
        for key in ("after", "before", "just_before"):
            value = reading.get(key)
            if value is None:
                continue
            for label, factor in (("", 1.0), (" in metres", FEET / scale), (" in square metres", FEET * FEET / (scale * scale))):
                if close(value * factor, float(revit)):
                    return key + label
        if reading.get("int") is not None and close(float(reading["int"]), float(revit)):
            return "int"
    return None


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    listing = "--list" in sys.argv
    probe, reference = args
    rows, parameters = [], []
    for line in open(probe, encoding="utf-8", errors="replace"):
        line = line.strip()
        if not line.startswith("{"):
            continue
        row = json.loads(line)
        if "parameters" in row:
            parameters = row["parameters"]
        elif row.get("id") is not None:
            rows.append(row)
    by_tag, by_global_id, scale = revit_values(reference)
    print(f"parameters named Pset_*: {len(parameters)}; elements carrying one: {len(rows)}")
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
            value, entity, where = revit[(set_name, prop)]
            how = matches(value, reading, scale)
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

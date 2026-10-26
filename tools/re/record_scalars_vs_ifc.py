#!/usr/bin/env python3
"""Which native record field tells a boolean property Revit's export writes
(RE-228, #35, W7).

Reads a probe's `rec <tag> <record class> <object>.<field>=<value> ...`
lines (`examples/probe_re228_wall_record_scalars.rs`,
`examples/probe_re233_column_record_scalars.rs`) and Revit's IFC
export. For every boolean property of every property set Revit writes on
those elements (by Tag), and for whether Revit writes a property at all
where it writes it on some of them only (`present:<set>.<property>`), and
for whether Revit writes the element's body as anything but extruded solids
(`body:not-extruded`, through mapped items: a mullion with cut ends), and
for whether Revit writes each base quantity (`present:<Qto set>.<quantity>`)
in the same way, and every field the probe printed, it takes the
one value of the field whose presence best predicts the property (the
property is true exactly where the field has that value, or exactly where
it has not) and counts how many elements that gets right. A field missing
from a record counts as the value `-`.

It prints, per property, the true and false counts and the 12 best fields
with their accuracy, perfect ones first.

`RECORD_FILTER=<field>=<value>[,<field>=<value>...]` keeps only the records
whose fields have those values (`-` for a field missing from the record),
to rank the fields within a group.

Usage:

    python3 tools/re/record_scalars_vs_ifc.py <probe.txt> <revit.ifc> [--list]
"""

import collections
import os
import sys

import ifcopenshell
import ifcopenshell.util.element


def read_probe(path):
    rows = {}
    for line in open(path, encoding="utf-8", errors="replace"):
        parts = line.split()
        if len(parts) < 3 or parts[0] != "rec" or parts[2] == "-":
            continue
        fields = {}
        for part in parts[3:]:
            name, _, value = part.partition("=")
            fields[name] = value
        rows[parts[1]] = fields
    return rows


def extruded_only(element):
    """Whether every item of the element's Body is an IfcExtrudedAreaSolid,
    through mapped items; None without a Body."""
    if element.Representation is None:
        return None
    kinds = []
    for rep in element.Representation.Representations:
        if rep.RepresentationIdentifier != "Body":
            continue
        items = list(rep.Items)
        while items:
            item = items.pop()
            if item.is_a("IfcMappedItem"):
                items.extend(item.MappingSource.MappedRepresentation.Items)
            else:
                kinds.append(item.is_a("IfcExtrudedAreaSolid"))
    return all(kinds) if kinds else None


def main():
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    rows = read_probe(args[0])
    for wanted in filter(None, os.environ.get("RECORD_FILTER", "").split(",")):
        name, _, value = wanted.partition("=")
        rows = {tag: fields for tag, fields in rows.items() if fields.get(name, "-") == value}
    revit = ifcopenshell.open(args[1])
    truths = collections.defaultdict(dict)
    seen = set()
    entity_of = {}
    present = collections.defaultdict(set)
    for element in revit.by_type("IfcElement"):
        tag = str(element.Tag) if element.Tag else None
        if tag not in rows:
            continue
        seen.add(tag)
        entity_of[tag] = element.is_a()
        extruded = extruded_only(element)
        if extruded is not None:
            truths["body:not-extruded"][tag] = not extruded
        for pset, props in ifcopenshell.util.element.get_psets(element, psets_only=True).items():
            for prop, value in props.items():
                if prop == "id":
                    continue
                present[f"{pset}.{prop}"].add(tag)
                if isinstance(value, bool):
                    truths[f"{pset}.{prop}"][tag] = value
        for qset, quantities in ifcopenshell.util.element.get_psets(element, qtos_only=True).items():
            for name in quantities:
                if name != "id":
                    present[f"{qset}.{name}"].add(tag)
    # Whether Revit writes a property or quantity at all, where it writes it
    # on some of the probe's elements of an entity and not on others (RE-233,
    # RE-247): among the elements of the entities it is on.
    for prop, tags in present.items():
        entities = {entity_of[t] for t in tags}
        universe = {t for t in seen if entity_of[t] in entities}
        if 0 < len(tags) < len(universe):
            truths[f"present:{prop}"] = {t: t in tags for t in universe}
    print(f"probe: {len(rows)} records; boolean properties Revit writes on them: {len(truths)}")
    names = sorted({name for fields in rows.values() for name in fields})
    for prop, by_tag in sorted(truths.items()):
        tags = sorted(by_tag)
        trues = sum(by_tag[t] for t in tags)
        print(f"{prop}: {len(tags)} elements, true {trues}, false {len(tags) - trues}")
        if trues in (0, len(tags)):
            print("  constant: no field needed")
            continue
        ranked = []
        for name in names:
            values = collections.Counter(rows[t].get(name, "-") for t in tags)
            if len(values) < 2:
                continue
            best = None
            for value in values:
                hits = sum((rows[t].get(name, "-") == value) == by_tag[t] for t in tags)
                score = max(hits, len(tags) - hits)
                sense = "=" if hits >= len(tags) - hits else "!="
                if best is None or score > best[0]:
                    best = (score, sense, value)
            ranked.append((best[0], name, best[1], best[2], len(values)))
        ranked.sort(key=lambda r: (-r[0], r[1]))
        for score, name, sense, value, distinct in ranked[:12]:
            print(f"  {score}/{len(tags)}  true where {name} {sense} {value}  ({distinct} values)")
        # The best field's values against the property, for a rule on a value.
        for _, name, _, _, _ in ranked[:3]:
            table = collections.Counter((rows[t].get(name, "-"), by_tag[t]) for t in tags)
            cells = ", ".join(f"{v}:{'T' if truth else 'F'} {n}" for (v, truth), n in sorted(table.items()))
            print(f"    {name}: {cells[:400]}")
    if "--list" in sys.argv:
        for prop, by_tag in sorted(truths.items()):
            for tag in sorted(by_tag)[:40]:
                print(f"row {prop} {tag} {by_tag[tag]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

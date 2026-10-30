#!/usr/bin/env python3
"""Score each step of Revit's `IsExternal` rule against Revit's export (RE-193).

`probes/examples/probe_re193_is_external.rs` prints one JSON line per element with
what rvt-rs reads for each step of the rule Revit's exporter applies
(`CategoryUtil.IsElementExternal` in revit-ifc, CLEANROOM.md source 5), the
first step that gives a value winning:

1. a Yes/No parameter named for the set Revit writes the value in
   (`Pset_WallCommon.IsExternal` for a wall), on the element, then its type,
   then one named IsExternal (revit-ifc's property set entry reads those
   names before its calculator);
2. the type's Function, external unless 0 (Interior): the type's
   `FUNCTION_PARAM` entry (RE-158), or a wall type's native `m_function`
   (RE-185);
3. a roof is external;
4. a family instance takes its host's value: here a door's or window's host
   wall and a mullion's or panel's curtain wall, by the calculator on it (a
   parameter named IsExternal, its type's Function, a roof, else not
   external);
5. otherwise the element is not external.

This joins each line to Revit's element of the same Tag, else the same
GlobalId (a space has no Tag), and reads the `IsExternal` of its `Pset_*Common`
set. Per IFC entity Revit writes, it counts the elements decided at each
step and how many of those equal Revit's value. Where the host gives no
value, the element is counted as undecided rather than defaulted. With
--list it prints the elements on which the rule and Revit differ.

Usage:
    python3 tools/re/is_external_vs_ifc.py PROBE.txt REVIT.ifc [--list]

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import collections
import json
import sys

import ifcopenshell
import ifcopenshell.util.element


def revit_values(path):
    """(Tag -> (entity, IsExternal), GlobalId -> (entity, IsExternal))."""
    model = ifcopenshell.open(path)
    by_tag, by_global_id = {}, {}
    for product in model.by_type("IfcProduct"):
        value, pset = None, None
        psets = ifcopenshell.util.element.get_psets(product, psets_only=True, should_inherit=False)
        for name, properties in psets.items():
            if name.startswith("Pset_") and name.endswith("Common") and "IsExternal" in properties:
                value, pset = properties["IsExternal"], name
        if value is None:
            continue
        record = (product.is_a(), bool(value), pset)
        tag = getattr(product, "Tag", None)
        if tag:
            by_tag[str(tag)] = record
        by_global_id[product.GlobalId] = record
    return by_tag, by_global_id


def parameter(named, names, wanted):
    """The Yes/No value of the parameter named `wanted` among `named`, or None."""
    for parameter_id, value in (named or {}).items():
        if names.get(str(parameter_id)) == wanted:
            return value != 0
    return None


def calculated(row, names):
    """The calculator's steps without the host: (step, value), or None."""
    for key in ("named", "type_named"):
        value = parameter(row.get(key), names, "IsExternal")
        if value is not None:
            return "IsExternal parameter", value
    function = row.get("function")
    if function is None:
        function = row.get("wall_function")
    if function is not None:
        return "type Function", function != 0
    if row.get("class") == "Roof":
        return "roof", True
    return None


def decide(row, pset, names):
    """(step, value) by the rule for the value Revit writes in `pset`."""
    for key, whose in (("named", "element's"), ("type_named", "type's")):
        value = parameter(row.get(key), names, f"{pset}.IsExternal")
        if value is not None:
            return f"1 {whose} {pset}.IsExternal", value
    own = calculated(row, names)
    if own is not None:
        return f"2-3 {own[0]}", own[1]
    for key, label in (("host", "host wall"), ("whole", "curtain wall")):
        related = row.get(key)
        if related:
            decided = calculated(related, names)
            if decided is None:
                return f"4 {label}: its default", False
            return f"4 {label}: {decided[0]}", decided[1]
    return "5 default", False


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    listing = "--list" in sys.argv
    probe_path, reference = args
    rows = []
    parameters = {}
    pset_parameters = []
    for line in open(probe_path, encoding="utf-8", errors="replace"):
        line = line.strip()
        if not line.startswith("{"):
            continue
        row = json.loads(line)
        if "parameters" in row:
            parameters = row["parameters"]
        elif "pset_parameters" in row:
            pset_parameters = row["pset_parameters"]
        elif row.get("id") is not None:
            rows.append(row)
    by_tag, by_global_id = revit_values(reference)
    print(f"Revit's export: {len(by_global_id)} elements with an IsExternal; probe: {len(rows)} elements")
    print(f"parameters named IsExternal or <set>.IsExternal: {parameters or 'none'}")
    print(f"parameters named Pset_*: {len(pset_parameters)}: {', '.join(pset_parameters[:80])}")

    tally = collections.Counter()
    seen = collections.Counter()
    values = collections.Counter()
    differing = []
    parts = collections.Counter()
    for row in rows:
        revit = by_tag.get(str(row["id"])) or by_global_id.get(row.get("global_id") or "")
        if revit is None:
            continue
        entity, value, pset = revit
        seen[entity] += 1
        values[(entity, value)] += 1
        step, decided = decide(row, pset, parameters)
        if decided is None:
            outcome = "undecided"
        elif decided == value:
            outcome = "equal"
        else:
            outcome = "different"
            differing.append((entity, value, step, row))
        tally[(entity, step, outcome)] += 1
        if entity in ("IfcMember", "IfcPlate"):
            whole = row.get("whole") or {}
            named = tuple(sorted((parameters.get(str(k), k), v) for k, v in (whole.get("named") or {}).items()))
            parts[(entity, value, whole.get("wall_function"), named, row.get("type"))] += 1

    for entity in sorted(seen):
        print(f"\n{entity}: {seen[entity]} paired; Revit true {values[(entity, True)]}, false {values[(entity, False)]}")
        steps = sorted({step for (e, step, _) in tally if e == entity})
        for step in steps:
            counts = {o: tally[(entity, step, o)] for o in ("equal", "different", "undecided")}
            print(f"  {step:44} equal {counts['equal']:5}  different {counts['different']:5}  undecided {counts['undecided']:5}")
    if listing:
        print("\nIfcMember and IfcPlate by Revit's value, their curtain wall's Function and set parameter,"
              " and their own type:")
        for (entity, value, wall_function, named, own_type), n in parts.most_common(40):
            print(f"  {n:6}  {entity} Revit {value}; curtain wall Function {wall_function}, its parameters {named}; type {own_type}")
        print(f"\n{len(differing)} where the rule and Revit differ:")
        for entity, value, step, row in differing[:80]:
            host = row.get("host") or row.get("whole")
            print(
                f"  {entity} {row['id']} {row['class']} type {row.get('type')}: Revit {value}; {step};"
                f" Function {row.get('function')} copies {row.get('function_copies')} wall {row.get('wall_function')};"
                f" own {row.get('own_function')}; named {row.get('named')} type named {row.get('type_named')};"
                f" related {host}"
            )


main()

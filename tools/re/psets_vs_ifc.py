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
differ, and how many are missing. Then, for the 40 properties most often
missing, the values Revit writes for them most often and the entities it
writes them on, so what a missing property would take is visible without
opening Revit's export (W7).

Usage:

    python3 tools/re/psets_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc> [--list N]

With `PSETS_MISSING=<set>.<property>[,...]` in the environment it also
lists, for each such property, every object Revit writes it on and rvt-rs
does not: its key, Revit's entity and value, and rvt-rs's entity and Name.

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import collections
import os
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
    if type_a in ("enumerated", "list"):
        # Item by item, so a list of lengths gets the same tolerance as one.
        return len(raw_a) == len(raw_b) and all(same(x, y) for x, y in zip(raw_a, raw_b))
    if isinstance(raw_a, float) and isinstance(raw_b, (int, float)):
        return abs(raw_a - raw_b) <= 1e-5 * max(1.0, abs(raw_a), abs(raw_b))
    return raw_a == raw_b


# IFC2X3's Pset_SpaceCommon holds a space's coverings; IFC4 moved them to
# Pset_SpaceCoveringRequirements, where an IFC4 export writes them
# (buildingSMART's IFC2X3 and IFC4 property set definitions).
IFC2X3_SPACE_COVERINGS = {
    "FloorCovering",
    "WallCovering",
    "CeilingCovering",
    "SkirtingBoard",
    "ConcealedFlooring",
    "ConcealedCeiling",
}

# An IFC2X3 export's Reference sets that IFC4 does not have (#822). Revit's
# IFC2X3 export writes an element's type name in Pset_QuantityTakeOff and, on
# a distribution element, Pset_DistributionFlowElementCommon; its IFC4 export
# writes it in Pset_EnvironmentalImpactIndicators and in the element's own
# type set, which IFC4 applies to the entity (BIM4LCA office).
IFC2X3_QUANTITY_TAKE_OFF = "Pset_QuantityTakeOff"
IFC2X3_FLOW_ELEMENT_COMMON = "Pset_DistributionFlowElementCommon"
IFC4_TYPE_SETS = {
    "IfcDuctSegment": "Pset_DuctSegmentTypeCommon",
    "IfcPipeSegment": "Pset_PipeSegmentTypeCommon",
    "IfcDuctFitting": "Pset_DuctFittingTypeCommon",
    "IfcPipeFitting": "Pset_PipeFittingTypeCommon",
    "IfcAirTerminal": "Pset_AirTerminalTypeCommon",
    "IfcSanitaryTerminal": "Pset_SanitaryTerminalTypeCommon",
    "IfcWasteTerminal": "Pset_WasteTerminalTypeCommon",
    "IfcFireSuppressionTerminal": "Pset_FireSuppressionTerminalTypeCommon",
    "IfcLightFixture": "Pset_LightFixtureTypeCommon",
    "IfcElectricAppliance": "Pset_ElectricApplianceTypeCommon",
    "IfcAlarm": "Pset_AlarmTypeCommon",
}
# IFC2X3's flow-segment sets (#822): IFC4 has none. A pipe's invert is in
# Pset_PipeSegmentOccurrence; a segment's length, and IFC2X3's
# Pset_DuctSegmentTypeCommon.Length, is in IFC4's base quantities, an
# IfcElementQuantity this scorer does not read, so those lengths are counted
# apart rather than scored.
IFC2X3_INVERT = ("Pset_FlowSegmentPipeSegment", "InvertElevation")
IFC2X3_LENGTHS = {
    ("Pset_FlowSegmentPipeSegment", "Length"),
    ("Pset_FlowSegmentDuctSegment", "Length"),
    ("Pset_DuctSegmentTypeCommon", "Length"),
}
# IFC2X3's Pset_PrecastConcreteElementGeneral.SerialNumber (#822): IFC4's set
# has none. Revit's IFC2X3 export writes the same value in
# Pset_ManufacturerOccurrence, which IFC4 keeps and which is scored; counted
# apart.
IFC2X3_TWICE = {("Pset_PrecastConcreteElementGeneral", "SerialNumber")}
# IFC2X3's Pset_PipeFittingTypeCommon.NominalDiameter (#822): IFC4's set has
# none, and Revit's IFC4 Reference View export writes no port set holding one,
# so rvt-rs writes it in its own set, which this scorer does not read; counted
# apart.
IFC2X3_OWN_SET = {("Pset_PipeFittingTypeCommon", "NominalDiameter")}


def properties(path):
    """(object key, set, property) -> value, the keys of every object, each
    object's entity by key, and whether the file is IFC2X3. An IFC2X3 file's
    space coverings, and its Pset_QuantityTakeOff.Reference, are keyed under
    their IFC4 set."""
    f = ifcopenshell.open(path)
    ifc2x3 = f.schema == "IFC2X3"
    metres_per_unit = ifcopenshell.util.unit.calculate_unit_scale(f)
    out = {}
    objects = set()
    entities = {}
    for product in f.by_type("IfcObject"):
        key = key_of(product)
        if key:
            objects.add(key)
            entities[key] = product.is_a()
    for rel in f.by_type("IfcRelDefinesByProperties"):
        pset = rel.RelatingPropertyDefinition
        if not pset.is_a("IfcPropertySet") or not (pset.Name or "").startswith("Pset_"):
            continue
        for obj in rel.RelatedObjects:
            key = key_of(obj)
            if not key:
                continue
            for prop in pset.HasProperties or ():
                name = pset.Name
                if ifc2x3 and name == "Pset_SpaceCommon" and prop.Name in IFC2X3_SPACE_COVERINGS:
                    name = "Pset_SpaceCoveringRequirements"
                if ifc2x3 and name == IFC2X3_QUANTITY_TAKE_OFF and prop.Name == "Reference":
                    name = "Pset_EnvironmentalImpactIndicators"
                if ifc2x3 and (name, prop.Name) == IFC2X3_INVERT:
                    name = "Pset_PipeSegmentOccurrence"
                if ifc2x3 and (name, prop.Name) in IFC2X3_LENGTHS:
                    name = "quantities:" + name
                if ifc2x3 and (name, prop.Name) in IFC2X3_TWICE:
                    name = "twice:" + name
                if ifc2x3 and (name, prop.Name) in IFC2X3_OWN_SET:
                    name = "own:" + name
                if prop.is_a("IfcPropertySingleValue"):
                    out[(key, name, prop.Name)] = value_of(prop.NominalValue, metres_per_unit)
                elif prop.is_a("IfcPropertyEnumeratedValue"):
                    values = tuple(
                        value_of(v, metres_per_unit) for v in prop.EnumerationValues or ()
                    )
                    out[(key, name, prop.Name)] = ("enumerated", values)
                elif prop.is_a("IfcPropertyListValue"):
                    values = tuple(value_of(v, metres_per_unit) for v in prop.ListValues or ())
                    out[(key, name, prop.Name)] = ("list", values)
    return out, objects, entities, ifc2x3


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
    (ours, our_objects, our_entities, _), (theirs, their_objects, their_entities, their_ifc2x3) = (
        properties(args[0]),
        properties(args[1]),
    )
    if their_ifc2x3:
        # A distribution element's IFC2X3 Reference is compared with the one
        # in the IFC4 type set of the entity rvt-rs writes it as.
        theirs = {
            (
                (key, IFC4_TYPE_SETS[our_entities[key]], prop)
                if pset == IFC2X3_FLOW_ELEMENT_COMMON
                and prop == "Reference"
                and our_entities.get(key) in IFC4_TYPE_SETS
                else (key, pset, prop)
            ): value
            for (key, pset, prop), value in theirs.items()
        }
    rows = collections.defaultdict(lambda: [0, 0, 0, 0])
    missing_values = collections.defaultdict(collections.Counter)
    missing_classes = collections.defaultdict(collections.Counter)
    worst = []
    as_quantities = 0
    twice = 0
    in_own_set = 0
    for (key, pset, prop), value in sorted(theirs.items()):
        if key not in our_objects:
            continue
        if pset.startswith("quantities:"):
            as_quantities += 1
            continue
        if pset.startswith("twice:"):
            twice += 1
            continue
        if pset.startswith("own:"):
            in_own_set += 1
            continue
        row = rows[(pset, prop)]
        row[0] += 1
        mine = ours.get((key, pset, prop), "missing")
        if same(mine, value):
            row[1] += 1
        elif mine == "missing":
            row[3] += 1
            missing_values[(pset, prop)][repr(value)[:60]] += 1
            missing_classes[(pset, prop)][their_entities.get(key, "-")] += 1
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
    if as_quantities:
        print(f"IFC2X3 lengths IFC4 holds as quantities, not scored here: {as_quantities}")
    if twice:
        print(f"IFC2X3 values also in an IFC4 set and scored there, not scored here: {twice}")
    if in_own_set:
        print(f"IFC2X3 values no IFC4 set Revit's export writes holds, in rvt-rs's own set, not scored here: {in_own_set}")
    only = collections.defaultdict(lambda: [0, 0])
    for key, pset, prop in ours:
        if (key, pset, prop) not in theirs:
            only[(pset, prop)][0 if key in their_objects else 1] += 1
    print("rvt-rs values Revit does not write, on objects Revit writes and on others:")
    for (pset, prop), (written, other) in sorted(only.items()):
        print(f"  {pset}.{prop}: {written}, {other}")
    print("Revit's most common values where rvt-rs writes none:")
    for (pset, prop), values in sorted(missing_values.items(), key=lambda kv: -sum(kv[1].values()))[:40]:
        common = ", ".join(f"{v} x{n}" for v, n in values.most_common(4))
        classes = ", ".join(f"{c} x{n}" for c, n in missing_classes[(pset, prop)].most_common(4))
        print(f"  {pset}.{prop}: {common}; on {classes}")
    for line in worst:
        print(line)
    wanted = [w for w in os.environ.get("PSETS_MISSING", "").split(",") if w]
    if wanted:
        names = object_names(args[0])
        for (key, pset, prop), value in sorted(theirs.items()):
            if f"{pset}.{prop}" in wanted and key in our_objects and (key, pset, prop) not in ours:
                print(
                    f"  missing {pset}.{prop} {key} Revit {their_entities.get(key, '-')} {value}"
                    f" rvt-rs {our_entities.get(key, '-')} {names.get(key, '-')!r}"
                )
    return 0


def object_names(path):
    f = ifcopenshell.open(path)
    return {key_of(o): o.Name for o in f.by_type("IfcObject") if key_of(o)}


if __name__ == "__main__":
    sys.exit(main(sys.argv))

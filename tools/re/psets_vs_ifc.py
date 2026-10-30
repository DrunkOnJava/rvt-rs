#!/usr/bin/env python3
"""Score the property sets in an rvt-rs IFC against Revit's own IFC export.

Research tool for #35 (parameters), `reports/element-framing/RE-153-room-finishes.md`
and `RE-154-pset-parameters.md`.

Every single-value, enumerated and list property of every property set
Revit's export relates to an object, or a type object holds itself
(`HasPropertySets`), is keyed by the object (an element by its `Tag`, a space by its
`Name`, a storey by its `Name`, the building as `building`, a type object as
`type:<Tag>`), the set's name and the property's name. For each key Revit writes, the rvt-rs export either
has the same value (value type and value as written, numbers within a
relative 1e-5, lengths in metres from each file's own length unit, plane
angles in radians from its angle unit), a different one, or none. rvt-rs's own sets
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

Where Revit's export writes no standard property set at all, as its export
of Core Interior (RE-239 §5), rvt-rs's values are counted apart rather than
as values Revit does not write, as qtos_vs_ifc.py counts quantities where
the export has none (RE-242). Where Revit's export is IFC2X3, an IFC4
property that IFC2X3's definition of its set does not have (IfcOpenShell's
buildingSMART templates) is counted apart too: that export cannot write it.

Where Revit's export writes `IsExternal` on its type objects too, as the
exporter that made the BIM4LCA office's exports does (a cache of element types
that gives every curtain wall's members of a type after the first wall's the
value false, RE-269), the `IsExternal` of the curtain members (IfcMember and
IfcPlate under an IfcCurtainWall) of those types and of the type objects is
counted apart, not as missing: those are the exporter's cache, not the model's
value, and rvt-rs does not replay it. rvt-rs writes the curtain members' own
IsExternal by revit-ifc's host rule, the value of exporters from IFC_v24.2.20
on (the maintainer, 2026-10-30, RE-288), so a member of the office the cache
bucket does not hold is scored against it, and so are RE1 Architecture's
(exporter 26.2), which the rule gives equal. The type objects' own values stay
counted apart: rvt-rs writes none on a member type, a later exporter's
IsElementExternal returning null for an element type.

What a project's own user-defined property set file places is out of W7's
scope (the maintainer, 2026-10-26, RE-239) and counted apart: in an IFC4
Reference View export, the glazing set on an element that is not a door or a
window, and the space occupancy set, which revit-ifc's certified list for that
view does not create, so only such a file writes them (RE-239 §1). The glazing
set on doors and windows, where IFC4 applies it and rvt-rs writes it, is scored.

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import collections
import os
import sys

import ifcopenshell
import ifcopenshell.util.element
import ifcopenshell.util.pset
import ifcopenshell.util.unit


def key_of(obj):
    if obj.is_a("IfcBuilding"):
        return "building"
    if obj.is_a("IfcSpace") or obj.is_a("IfcBuildingStorey"):
        return f"{obj.is_a()}:{obj.Name}"
    tag = getattr(obj, "Tag", None)
    return f"tag:{tag}" if tag else None


def property_key_of(obj):
    """key_of, but an opening is keyed by whether a door or window fills it.

    Revit's export writes several openings with one Tag, the ElementId of the
    element they belong to: a door drawn as an opening and filled by its door,
    and, in each further wall or door that element cuts, an opening with no
    filling that holds `Pset_OpeningElementCommon.Reference` and `FireExit`
    (the BIM4LCA office's 31 door openings, 69 curtain-wall cuts and 39 fire
    hose cabinets). Keyed by Tag alone, those values were scored against
    rvt-rs's filled opening, or its curtain wall, which Revit gives none."""
    if obj.is_a("IfcOpeningElement"):
        tag = getattr(obj, "Tag", None)
        if not tag:
            return None
        return f"opening:{tag}:{'filled' if obj.HasFillings else 'void'}"
    return key_of(obj)


def value_of(value, metres_per_unit, radians_per_unit=1.0):
    if value is None:
        return None
    raw = value.wrappedValue
    if "LengthMeasure" in value.is_a():
        raw = raw * metres_per_unit
    elif value.is_a("IfcPlaneAngleMeasure"):
        raw = raw * radians_per_unit
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
# apart here and scored by qtos_vs_ifc.py (#1016).
IFC2X3_INVERT = ("Pset_FlowSegmentPipeSegment", "InvertElevation")
# Values Revit's export writes that are known to be wrong (RE-235 §5): left out
# of rvt-rs's export, so counted apart here, not as missing. A stair's or
# flight's WalkingLineOffset is half the run's width in feet read as
# millimetres.
REVIT_DEFECTS = {
    ("Pset_StairFlightCommon", "WalkingLineOffset"),
    ("Pset_StairCommon", "WalkingLineOffset"),
}
# The IsExternal of a curtain mullion or panel and of its type object is the
# product of the exporter's cache of element types, not of the model (RE-269):
# Revit 24.0's IsElementExternal also evaluates a type, and caches it as not
# external, after the first curtain wall that uses it has had its members'
# sets created, so every later wall's members of that type read false. It is
# not replayed by rvt-rs (the maintainer, 2026-10-30), which writes the host
# rule's value on the members instead (RE-288), and counted apart here, in the
# same bucket as REVIT_DEFECTS. A curtain member is an IfcMember
# or IfcPlate aggregated under an IfcCurtainWall, and the export is one of that
# exporter when the member's type object holds the IsExternal itself, as an
# exporter that does not cache types (24.2.20 on, RE1's 26.2) leaves it out.
CURTAIN_MEMBER_ENTITIES = ("IfcMember", "IfcPlate")
CURTAIN_MEMBER_SETS = ("Pset_MemberCommon", "Pset_PlateCommon")


def curtain_cache_values(f, out):
    """The (object key, set, property) triples of `out` that are the exporter's
    type cache's IsExternal: of the curtain members whose type object holds
    one, and of those type objects (RE-269)."""
    members = {}
    for entity in CURTAIN_MEMBER_ENTITIES:
        for product in f.by_type(entity):
            parents = [r.RelatingObject for r in product.Decomposes or ()]
            key = key_of(product)
            if not key or not any(p is not None and p.is_a("IfcCurtainWall") for p in parents):
                continue
            kind = ifcopenshell.util.element.get_type(product)
            tag = getattr(kind, "Tag", None) if kind is not None else None
            members[key] = f"type:{tag}" if tag else None
    holding = {
        key
        for (key, pset, prop) in out
        if prop == "IsExternal" and pset in CURTAIN_MEMBER_SETS and key.startswith("type:")
    }
    types = {kind for kind in members.values() if kind in holding}
    keys = {key for key, kind in members.items() if kind in types} | types
    return {
        (key, pset, "IsExternal")
        for key in keys
        for pset in CURTAIN_MEMBER_SETS
        if (key, pset, "IsExternal") in out
    }


# What a project's own user-defined property set file places (RE-239), out of
# W7's scope (the maintainer, 2026-10-26): in an IFC4 Reference View export,
# revit-ifc's certified list of the sets that view creates
# (IFCCertifiedEntitiesAndPSets.json, CLEANROOM.md source 5) holds neither set,
# so only such a file writes them. IFC4 applies the glazing set to doors and
# windows, where rvt-rs writes it; there it is scored.
EXPORT_FILE_GLAZING = "Pset_DoorWindowGlazingType"
EXPORT_FILE_OCCUPANCY = "Pset_SpaceOccupancyRequirements"
GLAZED_ENTITIES = ("IfcDoor", "IfcWindow", "IfcDoorType", "IfcWindowType")


def export_file_values(f, out, entities):
    """The (object key, set, property) triples of `out` that a project's own
    property set file placed (RE-239): none unless `f` is an IFC4 Reference
    View export."""
    views = " ".join(f.header.file_description.description or ())
    if f.schema != "IFC4" or "ReferenceView" not in views:
        return set()
    return {
        (key, pset, prop)
        for (key, pset, prop) in out
        if pset == EXPORT_FILE_OCCUPANCY
        or (pset == EXPORT_FILE_GLAZING and entities.get(key) not in GLAZED_ENTITIES)
    }


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


def ifc2x3_holds(pset, prop):
    """Whether an IFC2X3 export can hold the IFC4 property `pset`.`prop`:
    buildingSMART's IFC2X3 definition of the set has it (IfcOpenShell's
    templates), or this scorer reads it from the IFC2X3 set that keeps it."""
    if (pset == "Pset_SpaceCoveringRequirements" and prop in IFC2X3_SPACE_COVERINGS) or (
        prop == "Reference" and (pset == "Pset_EnvironmentalImpactIndicators" or pset in IFC4_TYPE_SETS.values())
    ):
        return True
    if (pset, prop) == ("Pset_PipeSegmentOccurrence", IFC2X3_INVERT[1]):
        return True
    template = ifcopenshell.util.pset.get_template("IFC2X3").get_by_name(pset)
    return template is not None and any(t.Name == prop for t in template.HasPropertyTemplates or ())


def properties(path):
    """(object key, set, property) -> value, the keys of every object, each
    object's entity by key, and whether the file is IFC2X3. An IFC2X3 file's
    space coverings, and its Pset_QuantityTakeOff.Reference, are keyed under
    their IFC4 set."""
    f = ifcopenshell.open(path)
    ifc2x3 = f.schema == "IFC2X3"
    metres_per_unit = ifcopenshell.util.unit.calculate_unit_scale(f)
    # A plane angle is compared in radians, whatever unit each file writes.
    radians_per_unit = ifcopenshell.util.unit.calculate_unit_scale(f, "PLANEANGLEUNIT")
    out = {}
    objects = set()
    entities = {}
    for product in f.by_type("IfcObject"):
        key = property_key_of(product)
        if key:
            objects.add(key)
            entities[key] = product.is_a()
    def add(key, pset):
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
                out[(key, name, prop.Name)] = value_of(prop.NominalValue, metres_per_unit, radians_per_unit)
            elif prop.is_a("IfcPropertyEnumeratedValue"):
                values = tuple(
                    value_of(v, metres_per_unit, radians_per_unit) for v in prop.EnumerationValues or ()
                )
                out[(key, name, prop.Name)] = ("enumerated", values)
            elif prop.is_a("IfcPropertyListValue"):
                values = tuple(value_of(v, metres_per_unit, radians_per_unit) for v in prop.ListValues or ())
                out[(key, name, prop.Name)] = ("list", values)

    def standard(pset):
        return pset.is_a("IfcPropertySet") and (pset.Name or "").startswith("Pset_")

    for rel in f.by_type("IfcRelDefinesByProperties"):
        pset = rel.RelatingPropertyDefinition
        if not standard(pset):
            continue
        for obj in rel.RelatedObjects:
            key = property_key_of(obj)
            if key:
                add(key, pset)
    # A type object holds its sets itself (HasPropertySets, RE-210 §2), keyed
    # by its Tag, the type's ElementId (#1018).
    for kind in f.by_type("IfcTypeObject"):
        tag = getattr(kind, "Tag", None)
        if not tag:
            continue
        key = f"type:{tag}"
        objects.add(key)
        entities[key] = kind.is_a()
        for pset in kind.HasPropertySets or ():
            if standard(pset):
                add(key, pset)
    return out, objects, entities, ifc2x3, curtain_cache_values(f, out), export_file_values(f, out, entities)


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
    (ours, our_objects, our_entities, _, _, _), (theirs, their_objects, their_entities, their_ifc2x3, their_cache, their_file) = (
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
    defects = collections.Counter()
    cache_defects = collections.Counter()
    unwritten_openings = collections.Counter()
    from_file = collections.Counter()
    for (key, pset, prop), value in sorted(theirs.items()):
        if key not in our_objects:
            if key.startswith("opening:"):
                unwritten_openings[(pset, prop)] += 1
            continue
        if (key, pset, prop) in their_file:
            from_file[(pset, prop)] += 1
            continue
        if (key, pset, prop) in their_cache:
            cache_defects[(pset, prop, "types" if key.startswith("type:") else "curtain members")] += 1
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
        if (pset, prop) in REVIT_DEFECTS:
            defects[(pset, prop)] += 1
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
    # Revit's export writes no standard property set at all where it was made
    # with no property sets, as Core Interior's was (RE-239 §5):
    # what rvt-rs writes cannot be scored against it, so it is counted apart.
    setup_apart = extra if not theirs else 0
    extra -= setup_apart
    # An IFC2X3 export cannot write an IFC4 property IFC2X3's definition of its
    # set does not have, as RE1's exports by Revit 26.2 cannot: counted apart.
    beyond_ifc2x3 = collections.Counter(
        (pset, prop)
        for key, pset, prop in ours
        if theirs and their_ifc2x3 and (key, pset, prop) not in theirs and not ifc2x3_holds(pset, prop)
    )
    extra -= sum(beyond_ifc2x3.values())
    print(
        f"Revit values on objects rvt-rs writes: {totals[0]}; equal: {totals[1]}; "
        f"different: {totals[2]}; missing: {totals[3]}; rvt-rs values Revit does not write: {extra}"
    )
    for (pset, prop), (n, equal, different, missing) in sorted(rows.items()):
        print(f"  {pset}.{prop}: {n}, equal {equal}, different {different}, missing {missing}")
    for (pset, prop), n in sorted(defects.items()):
        print(f"Revit values known to be wrong, left out (RE-235 §5): {pset}.{prop} {n}")
    for (pset, prop, where), n in sorted(cache_defects.items()):
        print(f"Revit values known to be wrong, left out (RE-235 §5, RE-269): {pset}.{prop} on {where} {n}")
    if from_file:
        print(
            f"Revit values the project's own property set file places, out of scope (RE-239),"
            f" not scored: {sum(from_file.values())}"
        )
        for (pset, prop), n in sorted(from_file.items()):
            print(f"  {pset}.{prop}: {n}")
    if unwritten_openings:
        print(
            f"Revit values on openings rvt-rs does not write (a door or wall cut in a further host,"
            f" a curtain wall's or cabinet's cut), not scored: {sum(unwritten_openings.values())}"
        )
        for (pset, prop), n in sorted(unwritten_openings.items()):
            print(f"  {pset}.{prop}: {n}")
    if setup_apart:
        print(
            f"Revit's export writes no standard property set (its setup, RE-239 §5);"
            f" rvt-rs values not scored: {setup_apart}"
        )
    if beyond_ifc2x3:
        print(
            f"IFC4 properties IFC2X3's definitions do not have, which Revit's IFC2X3 export cannot write,"
            f" not scored: {sum(beyond_ifc2x3.values())}"
        )
        for (pset, prop), n in sorted(beyond_ifc2x3.items()):
            print(f"  {pset}.{prop}: {n}")
    if as_quantities:
        print(f"IFC2X3 lengths IFC4 holds as quantities, not scored here: {as_quantities}")
    if twice:
        print(f"IFC2X3 values also in an IFC4 set and scored there, not scored here: {twice}")
    if in_own_set:
        print(f"IFC2X3 values no IFC4 set Revit's export writes holds, in rvt-rs's own set, not scored here: {in_own_set}")
    only = collections.defaultdict(lambda: [0, 0])
    for key, pset, prop in ours if theirs else ():
        if (key, pset, prop) not in theirs and (pset, prop) not in beyond_ifc2x3:
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
    return {property_key_of(o): o.Name for o in f.by_type("IfcObject") if property_key_of(o)}


if __name__ == "__main__":
    sys.exit(main(sys.argv))

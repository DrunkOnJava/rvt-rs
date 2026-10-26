#!/usr/bin/env python3
"""Score the quantity sets in an rvt-rs IFC against Revit's own IFC export
(#1016, W7).

`psets_vs_ifc.py` scores property sets only. This scores every quantity of
every `Qto_` IfcElementQuantity Revit's export relates to an object, keyed
the same way: the object (an element by its `Tag`, a space or storey by its
`Name`, the building as `building`), the set's name and the quantity's
name. A value is its quantity entity and its value in SI units (metres,
square metres, cubic metres, kilograms, seconds) from each file's own unit
assignment; equal means the same entity and a value within a relative
1e-5, or within 1e-6 (a micrometre, a square or cubic one: the rvt-rs
writer's rounding) where that is larger. Revit's IFC2X3 export writes a
pipe's or duct's length in a property set (`psets_vs_ifc`'s
IFC2X3_LENGTHS), which is scored here as the IFC4 base quantity. A quantity carrying its own unit
is counted apart and not scored. Objects rvt-rs does not write at all are
left out. Where Revit's export writes no base quantities at all (no
`Qto_` IfcElementQuantity, nor IFC2X3's `BaseQuantities`), its export setup
had "Export base quantities" off (RE-242), and the quantities rvt-rs writes
are counted apart, not as values Revit does not write: `rvt-ifc
--no-base-quantities` leaves them out. A quantity Revit's export writes that
is known to be wrong (RE-235 §5, `REVIT_DEFECTS`) is left out of rvt-rs's
export and counted apart here, not as missing.

It prints the totals, a line per (set, quantity): how many values Revit
writes on objects rvt-rs also writes, how many are equal, how many differ
and how many are missing; the entities each missing quantity is on; the
values rvt-rs writes and Revit does not on objects both write, with up to
10 of their objects each and which of the set's quantities Revit gives it; and up to 40 differences, the largest relative ones
first.

Usage:

    python3 tools/re/qtos_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import collections
import os
import sys

import ifcopenshell
import ifcopenshell.util.unit

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from psets_vs_ifc import IFC2X3_LENGTHS, key_of  # noqa: E402

# Where an IFC2X3 export writes a segment's length (#822, psets_vs_ifc's
# IFC2X3_LENGTHS): the IFC4 base quantity of the entity rvt-rs writes it as.
IFC4_LENGTH_QUANTITY = {
    "IfcPipeSegment": "Qto_PipeSegmentBaseQuantities",
    "IfcDuctSegment": "Qto_DuctSegmentBaseQuantities",
}

# Quantities Revit's export writes that are known to be wrong (RE-235 §5), by
# set and quantity, with what they are wrong on: a predefined type of Revit's
# object, or a class of an object of the same key. A curtain mullion's
# CrossSectionArea grows with its length and is never its section's area; a
# door's opening's Width is its vertical extent and its Height its horizontal
# one, where IFC4 defines a wall opening's the other way (RE-244). An
# opening is keyed by the Tag of the door filling it.
REVIT_DEFECTS = {
    ("Qto_MemberBaseQuantities", "CrossSectionArea"): "MULLION",
    ("Qto_OpeningElementBaseQuantities", "Width"): "IfcDoor",
    ("Qto_OpeningElementBaseQuantities", "Height"): "IfcDoor",
}

KINDS = {
    "IfcQuantityLength": ("LengthValue", "LENGTHUNIT"),
    "IfcQuantityArea": ("AreaValue", "AREAUNIT"),
    "IfcQuantityVolume": ("VolumeValue", "VOLUMEUNIT"),
    "IfcQuantityWeight": ("WeightValue", "MASSUNIT"),
    "IfcQuantityTime": ("TimeValue", "TIMEUNIT"),
    "IfcQuantityCount": ("CountValue", None),
    "IfcQuantityNumber": ("NumberValue", None),
}


def scales(f):
    out = {}
    for unit in {u for _, u in KINDS.values() if u}:
        try:
            out[unit] = ifcopenshell.util.unit.calculate_unit_scale(f, unit)
        except Exception:
            out[unit] = None
    return out


def predefined_type(product):
    """An object's predefined type, or its type object's where its own is
    unset, NOTDEFINED or USERDEFINED."""
    own = getattr(product, "PredefinedType", None)
    if own not in (None, "NOTDEFINED", "USERDEFINED"):
        return own
    for rel in getattr(product, "IsTypedBy", None) or ():
        typed = getattr(rel.RelatingType, "PredefinedType", None)
        if typed not in (None, "NOTDEFINED", "USERDEFINED"):
            return typed
    return own


def quantities(path, entities_of=None):
    """(object key, set, quantity) -> (entity, SI value), the keys of every
    object, each object's entity by key, and how many quantities carry
    their own unit. In an IFC2X3 file, a segment length in a property set
    is keyed as the IFC4 base quantity of the entity `entities_of` (the
    other file's) gives the object."""
    f = ifcopenshell.open(path)
    unit_scale = scales(f)
    out = {}
    objects = set()
    entities = {}
    predefined = {}
    own_unit = 0
    if f.schema == "IFC2X3" and entities_of is not None:
        for rel in f.by_type("IfcRelDefinesByProperties"):
            definition = rel.RelatingPropertyDefinition
            if not definition.is_a("IfcPropertySet"):
                continue
            for obj in rel.RelatedObjects:
                key = key_of(obj)
                qset = IFC4_LENGTH_QUANTITY.get(entities_of.get(key))
                if qset is None:
                    continue
                for prop in definition.HasProperties or ():
                    if (definition.Name, prop.Name) in IFC2X3_LENGTHS and prop.is_a("IfcPropertySingleValue"):
                        raw = prop.NominalValue.wrappedValue if prop.NominalValue else None
                        scale = unit_scale.get("LENGTHUNIT")
                        value = None if raw is None or scale is None else float(raw) * scale
                        out.setdefault((key, qset, "Length"), ("IfcQuantityLength", value))
    for product in f.by_type("IfcObject"):
        key = key_of(product)
        if key:
            objects.add(key)
            entities[key] = product.is_a()
            predefined.setdefault(key, set()).update({predefined_type(product), product.is_a()})
    for rel in f.by_type("IfcRelDefinesByProperties"):
        definition = rel.RelatingPropertyDefinition
        if not definition.is_a("IfcElementQuantity") or not (definition.Name or "").startswith("Qto_"):
            continue
        for obj in rel.RelatedObjects:
            key = key_of(obj)
            if not key:
                continue
            for q in definition.Quantities or ():
                kind = KINDS.get(q.is_a())
                if kind is None:
                    continue
                if getattr(q, "Unit", None) is not None:
                    own_unit += 1
                    continue
                attribute, unit = kind
                raw = getattr(q, attribute)
                scale = unit_scale.get(unit) if unit else 1.0
                value = None if raw is None or scale is None else float(raw) * scale
                out[(key, definition.Name, q.Name)] = (q.is_a(), value)
    base = any(
        (q.Name or "").startswith("Qto_") or q.Name == "BaseQuantities" for q in f.by_type("IfcElementQuantity")
    )
    return out, objects, entities, own_unit, base, predefined


def relative(a, b):
    (_, x), (_, y) = a, b
    if x is None or y is None:
        return float("inf")
    return abs(x - y) / max(1e-9, abs(x), abs(y))


def same(a, b):
    (kind_a, x), (kind_b, y) = a, b
    if kind_a != kind_b or x is None or y is None:
        return kind_a == kind_b and x == y
    return abs(x - y) <= max(1e-6, 1e-5 * max(abs(x), abs(y)))


def main(argv):
    args = [a for a in argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    ours, our_objects, our_entities, our_own, _, _ = quantities(args[0])
    theirs, their_objects, their_entities, their_own, their_base, their_predefined = quantities(args[1], our_entities)
    rows = collections.defaultdict(lambda: [0, 0, 0, 0])
    missing_classes = collections.defaultdict(collections.Counter)
    differences = []
    defects = collections.Counter()
    for (key, qset, name), value in sorted(theirs.items()):
        if key not in our_objects:
            continue
        scope = REVIT_DEFECTS.get((qset, name))
        if scope is not None and scope in their_predefined.get(key, ()):
            defects[(qset, name)] += 1
            continue
        row = rows[(qset, name)]
        row[0] += 1
        mine = ours.get((key, qset, name))
        if mine is None:
            row[3] += 1
            missing_classes[(qset, name)][their_entities.get(key, "-")] += 1
        elif same(mine, value):
            row[1] += 1
        else:
            row[2] += 1
            differences.append((relative(mine, value), key, qset, name, mine, value))
    totals = [sum(r[i] for r in rows.values()) for i in range(4)]
    extra = collections.Counter(
        (qset, name) for key, qset, name in ours if key in their_objects and (key, qset, name) not in theirs
    )
    setting_off = collections.Counter() if their_base else extra
    if not their_base:
        extra = collections.Counter()
    print(
        f"Revit quantities on objects rvt-rs writes: {totals[0]}; equal: {totals[1]}; "
        f"different: {totals[2]}; missing: {totals[3]}; rvt-rs quantities Revit does not write: {sum(extra.values())}"
    )
    if not their_base:
        print(
            "Revit's export writes no base quantities (its setup's Export base quantities off, RE-242): "
            f"rvt-rs writes {sum(setting_off.values())}, which rvt-ifc --no-base-quantities leaves out"
        )
        for (qset, name), n in sorted(setting_off.items()):
            print(f"  {qset}.{name}: {n}")
    for (qset, name), n in sorted(defects.items()):
        print(f"Revit values known to be wrong, left out (RE-235 §5): {qset}.{name} {n}")
    if our_own or their_own:
        print(f"quantities with their own unit, not scored: rvt-rs {our_own}, Revit {their_own}")
    for (qset, name), (n, equal, different, missing) in sorted(rows.items()):
        print(f"  {qset}.{name}: {n}, equal {equal}, different {different}, missing {missing}")
    print("entities Revit writes each missing quantity on:")
    for (qset, name), classes in sorted(missing_classes.items(), key=lambda kv: -sum(kv[1].values())):
        on = ", ".join(f"{c} x{k}" for c, k in classes.most_common(4))
        print(f"  {qset}.{name}: {sum(classes.values())}; on {on}")
    print("rvt-rs quantities Revit does not write:")
    their_sets = collections.defaultdict(set)
    for key, qset, name in theirs:
        their_sets[(key, qset)].add(name)
    for (qset, name), n in sorted(extra.items()):
        print(f"  {qset}.{name}: {n}")
        keys = sorted(
            key for key, s, q in ours if (s, q) == (qset, name) and key in their_objects and (key, s, q) not in theirs
        )
        for key in keys[:10]:
            given = ", ".join(sorted(their_sets.get((key, qset), ()))) or "none"
            print(f"    {key} {their_entities[key]}: Revit's {qset}: {given}")
    print("largest differences (rvt-rs, Revit; SI units):")
    for _, key, qset, name, mine, value in sorted(differences, key=lambda d: -d[0])[:40]:
        print(f"  {key} {qset}.{name}: {mine[0]} {mine[1]}, {value[0]} {value[1]}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

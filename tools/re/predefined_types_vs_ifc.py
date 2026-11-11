#!/usr/bin/env python3
"""Score the PredefinedType rvt-rs writes against Revit's own IFC export, on every class that has one
(#323, RE-417, RE-419, #1692).

Pairs each IfcStair, IfcStairFlight, IfcSlab, IfcMember, IfcFurniture, IfcSolarDevice and
IfcTransportElement of the two files by Tag, and each IfcSpace that is a family instance by
GlobalId (Revit names it `<family>:<type>:<ElementId>`), and counts, per entity, how often the
PredefinedType is the same, differs, is unset in rvt-rs's, or is NOTDEFINED where Revit writes none
(RE-419: the default a furniture element takes), with Revit's values tallied, and names a few that
differ. Every other IfcElement class that declares a PredefinedType (an IfcWall, an IfcDoor, an
IfcCovering, an IfcBeam ...) is read the same way, by Tag, under "other classes", so that a rule on
a release (#1692: Revit 2027's export writes none on the classes above) is read against every
class both files hold and not only the eight, and the elements one file holds and the other does
not are counted by class. It also tallies the PredefinedType of every IfcElementType class of both
files (RE-419: a type is written once for each predefined type its instances carry) and of every
product of either that is no IfcElement and declares one (an IfcSpace, rooms and areas included, an
IfcDistributionPort ...), which carry no Tag to pair by. A Revit export in IFC2X3, which has no
PredefinedType on these classes, is not scored and says so.

Needs IfcOpenShell.

Usage:

    python3 tools/re/predefined_types_vs_ifc.py <rvt-rs.ifc> <revit.ifc>
"""

import collections
import re
import sys

import ifcopenshell

CLASSES = (
    "IfcStair",
    "IfcStairFlight",
    "IfcSlab",
    "IfcMember",
    "IfcFurniture",
    "IfcSolarDevice",
    "IfcTransportElement",
    "IfcSpace",
)
FAMILY_SPACE_NAME = re.compile(r":\d+$")


def declares_predefined(schema, cls, cache):
    """Whether the schema's entity `cls` has a PredefinedType attribute."""
    if cls not in cache:
        declaration = schema.declaration_by_name(cls)
        cache[cls] = any(a.name() == "PredefinedType" for a in declaration.all_attributes())
    return cache[cls]


def value_of(entity):
    return getattr(entity, "PredefinedType", None) or "-"


def predefined(model):
    out = {}
    for cls in CLASSES:
        for element in model.by_type(cls):
            if element.is_a() != cls:
                continue
            if cls == "IfcSpace":
                if element.Name and FAMILY_SPACE_NAME.search(element.Name):
                    out[(cls, element.GlobalId)] = value_of(element)
            elif element.Tag:
                out[(cls, str(element.Tag))] = value_of(element)
    return out


def predefined_other(model):
    """Every IfcElement of a class not in CLASSES whose entity declares a PredefinedType, by Tag."""
    schema = ifcopenshell.ifcopenshell_wrapper.schema_by_name(model.schema)
    has = {}
    out = {}
    for element in model.by_type("IfcElement"):
        cls = element.is_a()
        if cls in CLASSES or not element.Tag:
            continue
        if declares_predefined(schema, cls, has):
            out[(cls, str(element.Tag))] = value_of(element)
    return out


def type_values(model):
    """Per IfcElementType class that declares a PredefinedType, how many objects hold each value."""
    schema = ifcopenshell.ifcopenshell_wrapper.schema_by_name(model.schema)
    has = {}
    out = collections.defaultdict(collections.Counter)
    for entity in model.by_type("IfcElementType"):
        cls = entity.is_a()
        if declares_predefined(schema, cls, has):
            out[cls][value_of(entity)] += 1
    return out


def product_values(model):
    """Per class of product that is no IfcElement and declares a PredefinedType (an IfcSpace, an
    IfcDistributionPort ...), how many hold each value."""
    schema = ifcopenshell.ifcopenshell_wrapper.schema_by_name(model.schema)
    has = {}
    out = collections.defaultdict(collections.Counter)
    for product in model.by_type("IfcProduct"):
        cls = product.is_a()
        if not product.is_a("IfcElement") and declares_predefined(schema, cls, has):
            out[cls][value_of(product)] += 1
    return out


def report(cls, keys, ours, revit):
    """One line for a class: how rvt-rs's PredefinedType compares with Revit's on the elements both hold."""
    tally = collections.Counter()
    values = collections.Counter(revit[k] for k in keys)
    misses = []
    for key in keys:
        mine, theirs = ours[key], revit[key]
        if mine == theirs:
            tally["same"] += 1
        elif mine == "NOTDEFINED" and theirs == "-":
            tally["NOTDEFINED where Revit writes none"] += 1
            misses.append((key[1], mine, theirs))
        elif mine in ("-", "NOTDEFINED"):
            tally["unset in rvt-rs"] += 1
            misses.append((key[1], mine, theirs))
        else:
            tally["differs"] += 1
            misses.append((key[1], mine, theirs))
    print(
        f"{cls}: {len(keys)} in both; same {tally['same']}, differs {tally['differs']}, "
        f"unset in rvt-rs {tally['unset in rvt-rs']}, "
        f"NOTDEFINED where Revit writes none {tally['NOTDEFINED where Revit writes none']}; "
        f"Revit's: {dict(sorted(values.items()))}"
    )
    for tag, mine, theirs in misses[:6]:
        print(f"  {tag}: rvt-rs {mine}, Revit {theirs}")


def main():
    mine_model, revit_model = ifcopenshell.open(sys.argv[1]), ifcopenshell.open(sys.argv[2])
    if revit_model.schema == "IFC2X3":
        print("Revit's export is IFC2X3, which has no PredefinedType on these classes: not scored")
        return
    mine_types, revit_types = type_values(mine_model), type_values(revit_model)
    for cls in sorted(set(mine_types) | set(revit_types)):
        print(
            f"{cls} PredefinedType: rvt-rs {dict(sorted(mine_types[cls].items()))}, "
            f"Revit {dict(sorted(revit_types[cls].items()))}"
        )
    mine_products, revit_products = product_values(mine_model), product_values(revit_model)
    for cls in sorted(set(mine_products) | set(revit_products)):
        print(
            f"{cls} PredefinedType, all: rvt-rs {dict(sorted(mine_products[cls].items()))}, "
            f"Revit {dict(sorted(revit_products[cls].items()))}"
        )
    ours, revit = predefined(mine_model), predefined(revit_model)
    for cls in CLASSES:
        keys = [k for k in revit if k[0] == cls and k in ours]
        if keys:
            report(cls, keys, ours, revit)
    other_ours, other_revit = predefined_other(mine_model), predefined_other(revit_model)
    print("other classes (every IfcElement class with a PredefinedType not read above):")
    for cls in sorted({k[0] for k in other_revit}):
        keys = [k for k in other_revit if k[0] == cls and k in other_ours]
        if keys:
            report(cls, keys, other_ours, other_revit)
    only_revit = collections.Counter(k[0] for k in other_revit if k not in other_ours)
    only_ours = collections.Counter(k[0] for k in other_ours if k not in other_revit)
    print(
        f"  not paired by Tag: in Revit's only {dict(sorted(only_revit.items()))}, "
        f"in rvt-rs's only {dict(sorted(only_ours.items()))}"
    )


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Score rvt-rs's IFC type objects against Revit's own export (RE-110).

For every element Revit's export types through IfcRelDefinesByType,
keyed by its Tag (the ElementId), compare the type rvt-rs gives the
element with the same Tag: its entity, its Name (`Family:Type`), its Tag
(the type's ElementId) and its GlobalId.

rvt-rs writes IFC4. The RE1 exports are IFC2X3 (CoordinationView 2.0),
which has no IfcDoorType or IfcWindowType, only IfcDoorStyle and
IfcWindowStyle, and which types some elements by a supertype
(IfcDistributionElementType for sanitary terminals). An entity agrees
when ours is Revit's, its IFC4 counterpart, or a subtype of either in
IFC4 (B61).

Usage:
    python3 tools/re/type_objects_vs_ifc.py OURS.ifc REVIT.ifc
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.util.element

VERBOSE = "-v" in sys.argv
if VERBOSE:
    sys.argv.remove("-v")

IFC4 = ifcopenshell.ifcopenshell_wrapper.schema_by_name("IFC4")
# IFC2X3 type entities and their IFC4 counterparts.
IFC4_COUNTERPART = {"IfcDoorStyle": "IfcDoorType", "IfcWindowStyle": "IfcWindowType"}


def entity_agrees(revit, ours):
    """Whether `ours` (IFC4) is `revit`, its IFC4 counterpart, or a subtype."""
    wanted = {revit, IFC4_COUNTERPART.get(revit, revit)}
    try:
        declaration = IFC4.declaration_by_name(ours)
    except Exception:
        return ours in wanted
    while declaration is not None:
        if declaration.name() in wanted:
            return True
        declaration = declaration.supertype()
    return False


def typed(path):
    f = ifcopenshell.open(path)
    out = {}
    for rel in f.by_type("IfcRelDefinesByType"):
        t = rel.RelatingType
        for element in rel.RelatedObjects:
            tag = getattr(element, "Tag", None)
            if tag:
                out.setdefault(tag, (element.is_a(), t.is_a(), t.Name, t.Tag, t.GlobalId))
    return f, out


def main():
    ours_file, ours = typed(sys.argv[1])
    _, revit = typed(sys.argv[2])
    ours_tags = {e.Tag for e in ours_file.by_type("IfcElement") if getattr(e, "Tag", None)}
    fields = ["entity", "name", "tag", "global_id"]
    agree = collections.Counter()
    exported = both = 0
    by_class = collections.Counter()
    wrong = collections.Counter()
    shown = collections.Counter()
    for tag, (occurrence, *expected) in revit.items():
        if tag not in ours_tags:
            continue
        exported += 1
        got = ours.get(tag)
        if got is None:
            by_class[occurrence] += 1
            continue
        both += 1
        for field, e, g in zip(fields, expected, got[1:]):
            if e == g or (field == "entity" and entity_agrees(e, g)):
                agree[field] += 1
            else:
                wrong[(field, occurrence)] += 1
                if VERBOSE and shown[(field, occurrence)] < 3:
                    shown[(field, occurrence)] += 1
                    print(f"  {field} {occurrence} {tag}: Revit {e!r} ours {g!r}")
    extra = sum(1 for tag in ours if tag not in revit)
    print(f"Revit types {len(revit)} elements; {exported} of them are in ours")
    print(f"typed in ours: {both} of {exported}")
    for field in fields:
        print(f"  {field} agrees: {agree[field]} of {both}")
    print(f"typed in ours but not in Revit's export: {extra}")
    print(f"untyped in ours, by class: {dict(by_class)}")
    if wrong:
        print(f"disagreements: {dict(wrong)}")
    # RE-111: the TypeName property, which an element carries even where
    # rvt-rs does not know its family, against the type half of Revit's
    # type Name.
    type_names = {}
    for element in ours_file.by_type("IfcElement"):
        tag = getattr(element, "Tag", None)
        if not tag:
            continue
        for pset in ifcopenshell.util.element.get_psets(element).values():
            if pset.get("TypeName"):
                type_names.setdefault(tag, pset["TypeName"])
    carried = sum(1 for tag in revit if tag in ours_tags and tag in type_names)
    same = sum(
        1
        for tag, (_, _, name, *_) in revit.items()
        if tag in type_names and name.split(":", 1)[-1] == type_names[tag]
    )
    print(f"TypeName carried: {carried} of {exported}; equal to Revit's type: {same}")
    types = {t.Tag for t in ours_file.by_type("IfcTypeObject")}
    print(f"type objects in ours: {len(types)}")


if __name__ == "__main__":
    main()

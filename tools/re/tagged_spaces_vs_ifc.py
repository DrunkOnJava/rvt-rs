#!/usr/bin/env python3
"""What Revit writes on the `IfcSpace` that stand for family instances (#763).

Revit's exporter writes a room or an area as an `IfcSpace` with no `Tag`. A
family instance whose "Export to IFC As" names `IfcSpace` (RE-45) is written
as an `IfcSpace` that carries its ElementId as `Tag`. This lists, over those
spaces in Revit's export, how often each value occurs: `Name`, `LongName`,
`ObjectType`, `PredefinedType`, `CompositionType`, the object it is
aggregated in or contained in, its body, its type object, and its property
sets. It then counts what rvt-rs writes for the same Tag, by class.

Usage:
    python3 tools/re/tagged_spaces_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>

Needs IfcOpenShell (tested with 0.8.5).
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.util.element


def relation(product):
    """The object a product is aggregated in or contained in, by entity and name."""
    for kind, parent in (
        ("aggregated in", ifcopenshell.util.element.get_aggregate(product)),
        ("contained in", ifcopenshell.util.element.get_container(product)),
    ):
        if parent is not None:
            return f"{kind} {parent.is_a()}"
    return "in nothing"


def body(product):
    if not product.Representation:
        return "none"
    kinds = []
    for representation in product.Representation.Representations:
        items = ",".join(sorted({item.is_a() for item in representation.Items}))
        kinds.append(f"{representation.RepresentationIdentifier}/{representation.RepresentationType}:{items}")
    return " ".join(sorted(kinds))


def shape(text):
    """A name with its digits and family-type parts generalised, to count kinds."""
    if text is None:
        return "None"
    parts = str(text).split(":")
    if len(parts) == 3 and parts[2].isdigit():
        return "Family:Type:ElementId"
    return str(text)[:40]


def main():
    ours_path, revit_path = [a for a in sys.argv[1:] if not a.startswith("--")]
    revit = ifcopenshell.open(revit_path)
    ours = ifcopenshell.open(ours_path)
    spaces = [space for space in revit.by_type("IfcSpace") if space.Tag]
    print(f"Revit's export: {len(revit.by_type('IfcSpace'))} IfcSpace, {len(spaces)} with a Tag")
    census = collections.defaultdict(collections.Counter)
    for space in spaces:
        census["Name"][shape(space.Name)] += 1
        census["LongName"][shape(space.LongName)] += 1
        census["ObjectType"][shape(space.ObjectType)] += 1
        census["PredefinedType"][str(space.PredefinedType)] += 1
        census["CompositionType"][str(space.CompositionType)] += 1
        census["relation"][relation(space)] += 1
        census["body"][body(space)] += 1
        space_type = ifcopenshell.util.element.get_type(space)
        census["type object"][
            "none" if space_type is None else f"{space_type.is_a()} {space_type.PredefinedType} '{shape(space_type.Name)}'"
        ] += 1
        for set_name, values in ifcopenshell.util.element.get_psets(space).items():
            census["property set"][set_name] += 1
            for key, value in values.items():
                if key != "id":
                    census[f"{set_name}.{key}"][str(value)[:40]] += 1
    shown = ("Name", "LongName", "ObjectType", "PredefinedType", "CompositionType", "relation", "body",
             "type object", "property set")
    for field in shown:
        print(f"  {field}: " + ", ".join(f"{v} {n}" for v, n in census[field].most_common()))
    for field in sorted(census):
        if field not in shown:
            print(f"  {field}: " + ", ".join(f"{v} {n}" for v, n in census[field].most_common(6)))
    by_tag = {}
    for product in ours.by_type("IfcProduct"):
        tag = getattr(product, "Tag", None)
        if tag:
            by_tag.setdefault(str(tag), product)
    written = collections.Counter()
    for space in spaces:
        product = by_tag.get(str(space.Tag))
        written["not written" if product is None else f"{product.is_a()} {relation(product)}"] += 1
    print("rvt-rs writes the same Tags as: " + ", ".join(f"{v} {n}" for v, n in written.most_common()))


main()

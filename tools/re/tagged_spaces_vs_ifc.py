#!/usr/bin/env python3
"""What Revit writes on the `IfcSpace` that stand for family instances (#763).

Revit's exporter names a room's or an area's `IfcSpace` by its number. A
family instance whose "Export to IFC As" names `IfcSpace` (RE-45) is written
as an `IfcSpace` named `Family:Type:ElementId`, as Revit names other family
instances (an `IfcSpace` has no `Tag` in IFC4). This lists, over those
spaces in Revit's export, how often each value occurs: `Name`, `LongName`,
`ObjectType`, `PredefinedType`, `CompositionType`, the object it is
aggregated in or contained in, its body, its type object, and its property
sets. It then counts what rvt-rs writes for the same ElementId (its `Tag`,
or the id its space's `Name` ends with), by class.

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
    spaces = [space for space in revit.by_type("IfcSpace") if shape(space.Name) == "Family:Type:ElementId"]
    print(f"Revit's export: {len(revit.by_type('IfcSpace'))} IfcSpace, {len(spaces)} named Family:Type:ElementId")
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
        tag = getattr(product, "Tag", None) if product.is_a("IfcElement") else None
        if product.is_a("IfcSpace") and shape(product.Name) == "Family:Type:ElementId":
            tag = product.Name.split(":")[2]
        if tag:
            by_tag.setdefault(str(tag), product)
    written = collections.Counter()
    for space in spaces:
        product = by_tag.get(space.Name.split(":")[2])
        written["not written" if product is None else f"{product.is_a()} {relation(product)}"] += 1
    print("rvt-rs writes the same Tags as: " + ", ".join(f"{v} {n}" for v, n in written.most_common()))
    # #763: for each space rvt-rs also writes as an IfcSpace, which values agree.
    def type_of(space):
        space_type = ifcopenshell.util.element.get_type(space)
        return None if space_type is None else (space_type.is_a(), space_type.Name, space_type.PredefinedType)

    def common(space, key):
        return ifcopenshell.util.element.get_psets(space).get("Pset_SpaceCommon", {}).get(key)

    agree = collections.defaultdict(collections.Counter)
    for space in spaces:
        product = by_tag.get(space.Name.split(":")[2])
        if product is None or not product.is_a("IfcSpace"):
            continue
        for field, theirs, mine in (
            ("Name", space.Name, product.Name),
            ("LongName", space.LongName, product.LongName),
            ("ObjectType", space.ObjectType, product.ObjectType),
            ("PredefinedType", space.PredefinedType, product.PredefinedType),
            ("type object", type_of(space), type_of(product)),
            ("Pset_SpaceCommon.Reference", common(space, "Reference"), common(product, "Reference")),
            ("Pset_SpaceCommon.IsExternal", common(space, "IsExternal"), common(product, "IsExternal")),
            ("relation", relation(space), relation(product)),
        ):
            agree[field]["equal" if theirs == mine else f"differs ({str(theirs)[:40]} vs {str(mine)[:40]})"] += 1
    for field, counts in agree.items():
        print(f"  {field}: " + ", ".join(f"{v} {n}" for v, n in counts.most_common(4)))


main()

#!/usr/bin/env python3
"""Compare the element sets of an rvt-rs export with Revit's own, per IFC class (#763).

Pairs elements by `Tag` (the Revit ElementId) and spaces by `GlobalId`, leaving
out openings and virtual elements and their subtypes (an opening repeats its
door's or window's Tag). For every class of Revit's export it counts
how many of its elements rvt-rs writes as the same class, writes as another
class (named), or does not write at all; then the elements rvt-rs writes that
Revit's export lacks, per class. The ones written as another class are counted
by the category each export gives them (`Other.Category`) and any "Export to
IFC As" value they carry, which is what decides an element's class in Revit's
exporter (#792), and by the PredefinedType Revit writes on the element and on its
type object. It lists the missing and the extra ones, up to 200 per class,
each with its Name, ObjectType (LongName for a space), storey, category and
export value from the export that has it. Elements without a Tag cannot be
paired and are only counted, and so are elements that repeat a Tag already
seen (the first one is kept).

Usage:

    python3 tools/re/class_sets_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>

Needs IfcOpenShell (tested with 0.8.5).
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.util.element

LIST_LIMIT = 200
LEFT_OUT = ("IfcOpeningElement", "IfcVirtualElement")
# Revit's names for the parameters that set an element's IFC class, without
# spaces and lower-cased: its own "Export to IFC As", its type's "Export Type
# to IFC As", and the older shared parameters.
EXPORT_AS_NAMES = ("exporttoifcas", "exporttypetoifcas", "ifcexportas", "ifcexporttype")


def storey(product):
    """The name of the storey a product is contained in or part of, or "-"."""
    seen = 0
    while product is not None and seen < 16:
        if product.is_a("IfcBuildingStorey"):
            return product.Name or "-"
        product = ifcopenshell.util.element.get_container(product) or ifcopenshell.util.element.get_aggregate(product)
        seen += 1
    return "-"


def category_and_export_as(product):
    """The category an export gives a product and the "Export to IFC As" values it carries (its type's too)."""
    category, export_as = "-", []
    for name, properties in ifcopenshell.util.element.get_psets(product).items():
        for prop, value in properties.items():
            if name == "Other" and prop == "Category" and value:
                category = str(value)
            elif prop.replace(" ", "").lower() in EXPORT_AS_NAMES and value:
                export_as.append(f"{prop}={value}")
    return category, ";".join(sorted(set(export_as))) or "-"


def predefined_and_type(product):
    """A product's PredefinedType, and its type object's entity and PredefinedType ("-" where absent)."""
    own = getattr(product, "PredefinedType", None) or "-"
    type_object = ifcopenshell.util.element.get_type(product)
    if type_object is None:
        return own, "-", "-"
    return own, type_object.is_a(), getattr(type_object, "PredefinedType", None) or "-"


def members(model):
    """key -> (class, Name, ObjectType, storey, product), and the counts of untagged and repeated elements."""
    out, untagged, repeated = {}, 0, 0
    for element in model.by_type("IfcElement"):
        if any(element.is_a(left_out) for left_out in LEFT_OUT):
            continue
        if not element.Tag:
            untagged += 1
            continue
        key = ("Tag", str(element.Tag))
        if key in out:
            repeated += 1
            continue
        out[key] = (element.is_a(), element.Name or "-", element.ObjectType or "-", storey(element), element)
    for space in model.by_type("IfcSpace"):
        out[("GlobalId", space.GlobalId)] = ("IfcSpace", space.Name or "-", space.LongName or "-", storey(space), space)
    return out, untagged, repeated


def sort_key(key):
    kind, value = key
    return (kind, int(value) if value.isdigit() else 0, value)


def listing(title, keys, table):
    keys = sorted(keys, key=sort_key)
    print(f"{title} ({len(keys)}):")
    for key in keys[:LIST_LIMIT]:
        cls, name, object_type, level, product = table[key]
        category, export_as = category_and_export_as(product)
        print(f"  {key[1]}  {name!r}  {object_type!r}  storey {level!r}  category {category!r}  export as {export_as!r}")
    if len(keys) > LIST_LIMIT:
        print(f"  ... {len(keys) - LIST_LIMIT} more")


def main(argv):
    if len(argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    ours_file, revit_file = ifcopenshell.open(argv[0]), ifcopenshell.open(argv[1])
    ours, ours_untagged, ours_repeated = members(ours_file)
    revit, revit_untagged, revit_repeated = members(revit_file)
    print(f"Revit's export: {len(revit)} paired elements and spaces, {revit_untagged} untagged and {revit_repeated} repeated Tags left out")
    print(f"rvt-rs's export: {len(ours)} paired elements and spaces, {ours_untagged} untagged and {ours_repeated} repeated Tags left out")
    print(f"{'class':32} {'revit':>6} {'same':>6} {'other':>6} {'absent':>6}  other classes")
    missing = collections.defaultdict(set)
    for cls in sorted({r[0] for r in revit.values()}):
        keys = [k for k, r in revit.items() if r[0] == cls]
        same = sum(1 for k in keys if k in ours and ours[k][0] == cls)
        other = collections.Counter(ours[k][0] for k in keys if k in ours and ours[k][0] != cls)
        absent = [k for k in keys if k not in ours]
        missing[cls].update(absent)
        others = ", ".join(f"{c} {n}" for c, n in sorted(other.items()))
        print(f"{cls:32} {len(keys):6} {same:6} {sum(other.values()):6} {len(absent):6}  {others}")
    print("written as another class, by Revit's category and export value, and rvt-rs's:")
    for cls in sorted({r[0] for r in revit.values()}):
        groups = collections.Counter()
        sample = {}
        for key, record in revit.items():
            if record[0] != cls or key not in ours or ours[key][0] == cls:
                continue
            group = (
                ours[key][0],
                *category_and_export_as(record[4]),
                *predefined_and_type(record[4]),
                *category_and_export_as(ours[key][4]),
            )
            groups[group] += 1
            sample.setdefault(group, key[1])
        for group, n in sorted(groups.items(), key=lambda kv: -kv[1]):
            written, category, export_as, predefined, type_entity, type_predefined, our_category, our_export_as = group
            print(
                f"  {n:6}  {cls} -> {written}  Revit {category!r} {export_as!r}"
                f" PredefinedType {predefined} type {type_entity} {type_predefined}"
                f"  rvt-rs {our_category!r} {our_export_as!r}  e.g. {sample[group]}"
            )
    extra = collections.defaultdict(set)
    for key, record in ours.items():
        if key not in revit:
            extra[record[0]].add(key)
    print("extra in rvt-rs's export, by class:")
    for cls in sorted(extra):
        print(f"{cls:32} {len(extra[cls]):6}")
    for cls in sorted(missing):
        if missing[cls]:
            listing(f"missing {cls}", missing[cls], revit)
    for cls in sorted(extra):
        listing(f"extra {cls}", extra[cls], ours)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

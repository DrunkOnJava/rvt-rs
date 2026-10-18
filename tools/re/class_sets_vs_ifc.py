#!/usr/bin/env python3
"""Compare the element sets of an rvt-rs export with Revit's own, per IFC class (#763).

Pairs elements by `Tag` (the Revit ElementId) and spaces by `GlobalId`, leaving
out openings and virtual elements and their subtypes (an opening repeats its
door's or window's Tag). For every class of Revit's export it counts
how many of its elements rvt-rs writes as the same class, writes as another
class (named), or does not write at all; then the elements rvt-rs writes that
Revit's export lacks, per class. It lists the missing and the extra ones, up to
200 per class, each with its Name, ObjectType and storey from the export that
has it. Elements without a Tag cannot be paired and are only counted, and so
are elements that repeat a Tag already seen (the first one is kept).

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


def storey(product):
    """The name of the storey a product is contained in or part of, or "-"."""
    seen = 0
    while product is not None and seen < 16:
        if product.is_a("IfcBuildingStorey"):
            return product.Name or "-"
        product = ifcopenshell.util.element.get_container(product) or ifcopenshell.util.element.get_aggregate(product)
        seen += 1
    return "-"


def members(model):
    """key -> (class, Name, ObjectType, storey), and the counts of untagged and repeated elements."""
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
        out[key] = (element.is_a(), element.Name or "-", element.ObjectType or "-", storey(element))
    for space in model.by_type("IfcSpace"):
        out[("GlobalId", space.GlobalId)] = ("IfcSpace", space.Name or "-", space.LongName or "-", storey(space))
    return out, untagged, repeated


def sort_key(key):
    kind, value = key
    return (kind, int(value) if value.isdigit() else 0, value)


def listing(title, keys, table):
    keys = sorted(keys, key=sort_key)
    print(f"{title} ({len(keys)}):")
    for key in keys[:LIST_LIMIT]:
        cls, name, object_type, level = table[key]
        print(f"  {key[1]}  {name!r}  {object_type!r}  storey {level!r}")
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

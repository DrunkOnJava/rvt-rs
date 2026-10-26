#!/usr/bin/env python3
"""Which columns and curtain panels Revit's export gives a base quantity
(#1016, RE-247).

Revit's export gives most columns `Qto_ColumnBaseQuantities.Length` and most
curtain panels `Qto_PlateBaseQuantities.Width`, but not all. For every
IfcColumn and IfcPlate both files write (by Tag), it tallies whether Revit
writes the quantity against the kinds of item Revit draws the body with
(through mapped items; a boolean result counts as its kind and its first
operand's), the BodySource rvt-rs gives it and Revit's type name, and lists
up to 40 objects of each class without it: the quantities Revit does give
it, its body's items and rvt-rs's BodySource.

Usage:

    python3 tools/re/qto_presence_list.py <rvt-rs.ifc> <revit-export.ifc>
"""

import collections
import os
import sys

import ifcopenshell

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from qtos_vs_ifc import quantities  # noqa: E402
from wall_qtos_list import body_source  # noqa: E402

WATCHED = {
    "IfcColumn": ("Qto_ColumnBaseQuantities", "Length"),
    "IfcPlate": ("Qto_PlateBaseQuantities", "Width"),
}


def kinds(product):
    found = []
    if product.Representation is None:
        return "-"
    for rep in product.Representation.Representations:
        if rep.RepresentationIdentifier != "Body":
            continue
        items = list(rep.Items)
        while items:
            item = items.pop()
            if item.is_a("IfcMappedItem"):
                items.extend(item.MappingSource.MappedRepresentation.Items)
                continue
            if item.is_a("IfcBooleanResult"):
                found.append(f"{item.is_a()}({item.FirstOperand.is_a()})")
                continue
            found.append(item.is_a())
    return "+".join(sorted(found)) or "-"


def main(argv):
    args = [a for a in argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    ours_f, theirs_f = ifcopenshell.open(args[0]), ifcopenshell.open(args[1])
    sources = {f"tag:{p.Tag}": body_source(p) for c in WATCHED for p in ours_f.by_type(c) if p.Tag}
    theirs, *_ = quantities(args[1])
    given = collections.defaultdict(set)
    for key, qset, name in theirs:
        given[(key, qset)].add(name)
    for cls, (qset, name) in WATCHED.items():
        tally = collections.Counter()
        without = []
        for product in theirs_f.by_type(cls):
            key = f"tag:{product.Tag}" if product.Tag else None
            if key not in sources:
                continue
            has = name in given.get((key, qset), ())
            body = kinds(product)
            object_type = (getattr(product, "ObjectType", None) or "-").replace(" ", "_")
            tally[(has, body, sources[key])] += 1
            tally[(has, "type", object_type)] += 1
            if not has:
                others = ",".join(sorted(given.get((key, qset), ()))) or "none"
                without.append(f"    {key} {object_type} Revit {qset}: {others}; Revit body {body}; rvt-rs {sources[key]}")
        print(f"{cls} {qset}.{name}: Revit writes it on {sum(n for (h, b, _), n in tally.items() if h and b != 'type')} of "
              f"{sum(n for (_, b, _), n in tally.items() if b != 'type')} both files write")
        for (has, body, source), n in sorted(tally.items(), key=lambda kv: (kv[0][1] == "type", str(kv[0]))):
            label = f"type {source}" if body == "type" else f"{body} [{source}]"
            print(f"  {'with' if has else 'without'} {label}: {n}")
        for line in sorted(without)[:40]:
            print(line)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

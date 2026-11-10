"""Which Qto_ColumnBaseQuantities Revit writes on a column (the extrusion-derived ones: GrossVolume, Length,
CrossSectionArea, OuterSurfaceArea) against how Revit's export draws the column: body item classes, whether
it is voided, and whether rvt-rs draws it from a mesh. Counts only.

Usage: column_set_kind_vs_ifc.py <rvt-rs.ifc> <revit.ifc>
"""
import collections
import sys

import ifcopenshell
import ifcopenshell.util.element

EXTR = ("GrossVolume", "Length", "CrossSectionArea", "OuterSurfaceArea")

ours = ifcopenshell.open(sys.argv[1])
theirs = ifcopenshell.open(sys.argv[2])
our_item = {}
for c in ours.by_type("IfcColumn"):
    items = set()
    if c.Representation:
        for r in c.Representation.Representations:
            for i in r.Items:
                items.add(i.is_a())
    our_item[c.Tag] = "+".join(sorted(items)) or "none"
tally = collections.Counter()
for c in theirs.by_type("IfcColumn"):
    q = ifcopenshell.util.element.get_psets(c, qtos_only=True).get("Qto_ColumnBaseQuantities")
    if q is None:
        continue
    present = tuple(n for n in EXTR if n in q) or ("none of the four",)
    items = set()
    if c.Representation:
        for r in c.Representation.Representations:
            if r.RepresentationIdentifier == "Body":
                for i in r.Items:
                    if i.is_a("IfcMappedItem"):
                        # The type's own representation: descend one level.
                        for mr in i.MappingSource.MappedRepresentation.Items:
                            items.add("mapped " + mr.is_a())
                            if mr.is_a("IfcBooleanResult") or mr.is_a("IfcBooleanClippingResult"):
                                items.add("mapped boolean")
                    else:
                        items.add(i.is_a())
                    if i.is_a("IfcBooleanResult") or i.is_a("IfcBooleanClippingResult"):
                        items.add("boolean")
    voided = bool(getattr(c, "HasOpenings", None))
    tally[("Revit writes " + "+".join(present), "Revit body " + ("+".join(sorted(items)) or "none"), "voided" if voided else "unvoided", "rvt-rs body " + our_item.get(c.Tag, "absent"))] += 1
print("Revit columns with Qto_ColumnBaseQuantities, by which extrusion-derived quantities it writes, Revit's body, openings, rvt-rs's body:")
for k, n in sorted(tally.items()):
    print("  ", n, *k)

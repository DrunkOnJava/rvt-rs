"""Which kind of Qto_WallBaseQuantities Revit writes on a wall (the full five, or NetVolume alone) against
how Revit's export draws the wall: its body item classes, whether it is voided, its IFC class.

revit-ifc's wall exporter writes the full set only on a wall it exports as an extrusion with an axis
(WallExporter: exportedAsWallWithAxis) and a wall exported as a BRep gets the volume alone from the
quantity definitions. This tests that reading on Revit's own exports. Counts only.

Usage: wall_set_kind_vs_ifc.py <rvt-rs.ifc> <revit.ifc>
"""
import collections
import sys

import ifcopenshell
import ifcopenshell.util.element

FULL = {"GrossSideArea", "GrossVolume", "Height", "NetSideArea", "NetVolume"}

theirs = ifcopenshell.open(sys.argv[2])
ours = ifcopenshell.open(sys.argv[1])
our_tags = {w.Tag for w in ours.by_type("IfcWall")}
tally = collections.Counter()
for wall in theirs.by_type("IfcWall"):
    q = ifcopenshell.util.element.get_psets(wall, qtos_only=True).get("Qto_WallBaseQuantities")
    if not q:
        continue
    names = {k for k in q if k != "id"}
    kind = "full" if FULL <= names else ("volume only" if "NetVolume" in names and not (names & {"NetSideArea", "GrossSideArea", "GrossVolume"}) else "other")
    items = set()
    if wall.Representation:
        for rep in wall.Representation.Representations:
            if rep.RepresentationIdentifier == "Body":
                for item in rep.Items:
                    items.add(item.is_a())
                    if item.is_a("IfcBooleanResult") or item.is_a("IfcBooleanClippingResult"):
                        items.add("boolean")
    voided = bool(getattr(wall, "HasOpenings", None))
    tally[(kind, wall.is_a(), "+".join(sorted(items)) or "none", "voided" if voided else "unvoided", "in rvt-rs" if wall.Tag in our_tags else "not in rvt-rs")] += 1
print("Revit walls with Qto_WallBaseQuantities, by quantity set kind, class, body item classes, openings:")
for key, n in sorted(tally.items()):
    print("  ", n, *key)

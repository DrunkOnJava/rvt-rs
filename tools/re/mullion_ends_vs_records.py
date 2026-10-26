#!/usr/bin/env python3
"""Which native record field tells how many of a curtain mullion's ends
Revit's NetSurfaceArea counts (#1016, W7).

revit-ifc's `NetSurfaceAreaCalculator` (CLEANROOM.md source 5) sums the
faces of the element's Revit solid, and Revit drops the faces where joined
elements meet: a mullion drawn as one extrusion of its section has its
section's perimeter times its length plus 0, 1 or 2 ends. This takes, for
each IfcMember of Revit's export whose body is one extruded solid, that count
from Revit's own values ((NetSurfaceArea - perimeter x depth) / area of its
solid), and whether its solid runs upright (through its placement, any mapped
item's transformation and the solid's position), and ranks every field of
`examples/probe_re241_mullion_record_scalars.rs`'s records against each
count, upright and not, with `record_scalars_vs_ifc.py`'s ranking.

Usage:

    python3 tools/re/mullion_ends_vs_records.py <probe.txt> <revit.ifc> [--list]
"""

import os
import sys

import ifcopenshell
import ifcopenshell.util.placement
import ifcopenshell.util.unit
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from qto_candidates_list import profile_measures  # noqa: E402
from qtos_vs_ifc import quantities  # noqa: E402
from record_scalars_vs_ifc import read_probe, report  # noqa: E402
from wall_qtos_list import solids  # noqa: E402


def mapping_matrix(item):
    """The 4x4 transformation an IfcMappedItem applies to its representation:
    its target operator after the inverse of its source's origin."""
    util = getattr(ifcopenshell.util.placement, "get_mappeditem_transformation", None)
    if util is not None:
        return np.array(util(item), dtype=float)
    target = np.eye(4)
    op = item.MappingTarget
    if op is not None:
        axis3 = getattr(op, "Axis3", None)
        x = np.array(op.Axis1.DirectionRatios if op.Axis1 else (1.0, 0.0, 0.0), dtype=float)
        z = np.array(axis3.DirectionRatios if axis3 else (0.0, 0.0, 1.0), dtype=float)
        y = np.array(op.Axis2.DirectionRatios, dtype=float) if op.Axis2 else np.cross(z, x)
        axes = [v / (np.linalg.norm(v) or 1) for v in (x, y, z)]
        target[:3, :3] = np.column_stack(axes) * (op.Scale or 1.0)
        target[:3, 3] = op.LocalOrigin.Coordinates
    origin = np.array(ifcopenshell.util.placement.get_axis2placement(item.MappingSource.MappingOrigin), dtype=float)
    return target @ np.linalg.inv(origin)


def main():
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    rows = read_probe(args[0])
    revit = ifcopenshell.open(args[1])
    scale = ifcopenshell.util.unit.calculate_unit_scale(revit)
    nsa = {}
    for (key, qset, name), (_, value) in quantities(args[1])[0].items():
        if qset == "Qto_MemberBaseQuantities" and name == "NetSurfaceArea" and value is not None:
            nsa[key.removeprefix("tag:")] = value
    counted = {}
    for member in revit.by_type("IfcMember"):
        tag = str(member.Tag) if member.Tag else None
        if tag not in rows or tag not in nsa or member.Representation is None:
            continue
        items = []
        for rep in member.Representation.Representations:
            if rep.RepresentationIdentifier == "Body":
                pending = [(item, np.eye(4)) for item in rep.Items]
                while pending:
                    item, mapped = pending.pop()
                    if item.is_a("IfcMappedItem"):
                        inner = mapped @ mapping_matrix(item)
                        pending.extend((i, inner) for i in item.MappingSource.MappedRepresentation.Items)
                        continue
                    items.append((item, mapped))
        found = [(s, mapped) for item, mapped in items for s in solids(item)]
        if len(found) != 1 or len(items) != 1:
            continue
        solid, mapped = found[0]
        m = profile_measures(solid.SweptArea, scale)
        if m is None or m[0] <= 0:
            continue
        area, perimeter = m[0], m[1]
        depth = solid.Depth * scale
        k = (nsa[tag] - perimeter * depth) / area
        if abs(k - round(k)) > 1e-3 or round(k) not in (0, 1, 2):
            continue
        place = np.array(ifcopenshell.util.placement.get_local_placement(member.ObjectPlacement)) @ mapped
        if solid.Position is not None:
            place = place @ np.array(ifcopenshell.util.placement.get_axis2placement(solid.Position))
        d = place[:3, :3] @ np.array(solid.ExtrudedDirection.DirectionRatios, dtype=float)
        upright = abs(d[2]) / (np.linalg.norm(d) or 1) > 0.999
        counted[tag] = (int(round(k)), upright)
    truths = {}
    for upright in (True, False):
        group = {t: k for t, (k, u) in counted.items() if u == upright}
        label = "upright" if upright else "not-upright"
        for want in (0, 1, 2):
            truths[f"ends={want}:{label}"] = {t: k == want for t, k in group.items()}
    report(rows, truths)
    return 0


if __name__ == "__main__":
    sys.exit(main())

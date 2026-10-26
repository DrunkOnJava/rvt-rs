#!/usr/bin/env python3
"""Which way Revit's opening base quantities point (#1016, W7).

For every filled IfcOpeningElement Revit's export gives
`Qto_OpeningElementBaseQuantities` (keyed by the Tag of the door or window
filling it), it finds the world direction of its body's profile X axis and
extrusion (vertical or horizontal), and whether rvt-rs's opening of the same
key has the same three extents (Width, Height and Depth in any order, a
relative 1e-5 or 1e-6 absolute). It then counts, for each of Width, Height
and Depth, how often it is the vertical extent, the horizontal one along
the extrusion, or the other horizontal one, and how often rvt-rs's opening
has that extent in the same direction. Lists up to 12 openings whose
extents differ.

Usage:

    python3 tools/re/opening_axes_list.py <rvt-rs.ifc> <revit-export.ifc>
"""

import collections
import os
import sys

import ifcopenshell
import ifcopenshell.util.placement
import ifcopenshell.util.unit
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from opening_qtos_list import extents  # noqa: E402
from wall_qtos_list import solids  # noqa: E402


def axis(placement):
    m = np.eye(4)
    if placement is not None:
        m = np.array(ifcopenshell.util.placement.get_axis2placement(placement))
    return m


def directions(f):
    """filler key -> (vertical, extrusion-horizontal, other-horizontal) extents and the qto."""
    scale = ifcopenshell.util.unit.calculate_unit_scale(f)
    out = {}
    for rel in f.by_type("IfcRelFillsElement"):
        opening, fill = rel.RelatingOpeningElement, rel.RelatedBuildingElement
        if not fill.Tag or opening.Representation is None:
            continue
        found = [
            s
            for rep in opening.Representation.Representations
            if rep.RepresentationIdentifier == "Body"
            for item in rep.Items
            for s in solids(item)
        ]
        if len(found) != 1:
            continue
        solid = found[0]
        x, y = extents(solid.SweptArea, scale)
        if x is None:
            continue
        world = np.array(ifcopenshell.util.placement.get_local_placement(opening.ObjectPlacement)) @ axis(solid.Position)
        rot = world[:3, :3]
        dirs = {
            "x": rot @ np.array([1.0, 0, 0]),
            "y": rot @ np.array([0, 1.0, 0]),
            "depth": rot @ np.array(solid.ExtrudedDirection.DirectionRatios, dtype=float),
        }
        sizes = {"x": x, "y": y, "depth": solid.Depth * scale}
        roles = {}
        for name, d in dirs.items():
            d = d / (np.linalg.norm(d) or 1)
            roles[name] = "vertical" if abs(d[2]) > 0.99 else ("horizontal" if abs(d[2]) < 0.01 else "slanted")
        qto = {}
        for r in getattr(opening, "IsDefinedBy", None) or ():
            if r.is_a("IfcRelDefinesByProperties") and r.RelatingPropertyDefinition.is_a("IfcElementQuantity"):
                for q in r.RelatingPropertyDefinition.Quantities or ():
                    if q.is_a("IfcQuantityLength"):
                        qto[q.Name] = float(q.LengthValue) * ifcopenshell.util.unit.calculate_unit_scale(f, "LENGTHUNIT")
        out[f"tag:{fill.Tag}"] = (sizes, roles, qto, fill.is_a())
    return out


def host_widths(f):
    """filler key -> its opening's host's Qto_WallBaseQuantities.Width, SI."""
    lengths = ifcopenshell.util.unit.calculate_unit_scale(f, "LENGTHUNIT")
    width_of = {}
    for rel in f.by_type("IfcRelDefinesByProperties"):
        d = rel.RelatingPropertyDefinition
        if d.is_a("IfcElementQuantity") and d.Name == "Qto_WallBaseQuantities":
            for q in d.Quantities or ():
                if q.Name == "Width" and q.is_a("IfcQuantityLength"):
                    for obj in rel.RelatedObjects:
                        width_of[obj.id()] = float(q.LengthValue) * lengths
    host_of = {rel.RelatedOpeningElement.id(): rel.RelatingBuildingElement for rel in f.by_type("IfcRelVoidsElement")}
    out = {}
    for rel in f.by_type("IfcRelFillsElement"):
        host = host_of.get(rel.RelatingOpeningElement.id())
        if rel.RelatedBuildingElement.Tag and host is not None:
            out[f"tag:{rel.RelatedBuildingElement.Tag}"] = (host.is_a(), width_of.get(host.id()))
    return out


def same(a, b):
    return a is not None and b is not None and abs(a - b) <= max(1e-6, 1e-5 * max(abs(a), abs(b)))


def main(argv):
    args = [a for a in argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    ours_f, theirs_f = ifcopenshell.open(args[0]), ifcopenshell.open(args[1])
    mine, theirs = directions(ours_f), directions(theirs_f)
    rows = {k: v for k, v in theirs.items() if all(n in v[2] for n in ("Width", "Height", "Depth"))}
    print(f"filled openings with Width, Height and Depth in Revit's export: {len(rows)}; rvt-rs writes {sum(k in mine for k in rows)}")
    role_of = collections.Counter()
    same_box = 0
    differ = []
    agree = collections.Counter()
    for k, (sizes, roles, qto, cls) in sorted(rows.items()):
        for name in ("Width", "Height", "Depth"):
            hit = [a for a in sizes if same(qto[name], sizes[a])]
            role_of[(cls, name, "+".join(sorted(roles[a] + ("-extrusion" if a == "depth" else "") for a in hit)) or "none")] += 1
        if k not in mine:
            continue
        ours_sizes, ours_roles, _, _ = mine[k]
        if all(any(same(q, s) for s in ours_sizes.values()) for q in (qto["Width"], qto["Height"], qto["Depth"])):
            same_box += 1
            for name in ("Width", "Height", "Depth"):
                theirs_role = [roles[a] for a in sizes if same(qto[name], sizes[a])]
                ours_role = [ours_roles[a] for a in ours_sizes if same(qto[name], ours_sizes[a])]
                agree[(name, bool(set(theirs_role) & set(ours_role)))] += 1
        elif len(differ) < 12:
            differ.append(f"{k} {cls}: Revit W {qto['Width']:.5g} H {qto['Height']:.5g} D {qto['Depth']:.5g}; rvt-rs " + " x ".join(f"{v:.5g}" for v in ours_sizes.values()))
    revit_hosts, ours_hosts = host_widths(theirs_f), host_widths(ours_f)
    depth_vs = collections.Counter()
    for k, (_, _, qto, _) in sorted(rows.items()):
        host, width = revit_hosts.get(k, ("-", None))
        _, our_width = ours_hosts.get(k, ("-", None))
        depth_vs[(host, "Revit's host Width", same(qto["Depth"], width))] += 1
        depth_vs[(host, "rvt-rs's host Width", same(qto["Depth"], our_width))] += 1
    print("Depth = the host's Qto_WallBaseQuantities.Width (host class, whose, equal):")
    for key, n in sorted(depth_vs.items(), key=str):
        print(f"    {key[0]} {key[1]} {key[2]} x{n}")
    print("which of Revit's body extents each quantity is (class, quantity, direction):")
    for key, n in sorted(role_of.items()):
        print(f"    {key[0]} {key[1]}: {key[2]} x{n}")
    print(f"rvt-rs's opening has the same three extents: {same_box}")
    print("and in the same direction (quantity, agrees):")
    for key, n in sorted(agree.items()):
        print(f"    {key[0]} {key[1]} x{n}")
    print("rvt-rs's extents differ:")
    for line in differ:
        print(f"    {line}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

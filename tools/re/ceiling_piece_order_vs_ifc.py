#!/usr/bin/env python3
"""Which body item of a ceiling in pieces gives Revit's `GrossArea` (#1209)?

revit-ifc 24.2's `BodyExporter.ExportBody` (CLEANROOM.md source 5,
`Exporter/BodyExporter.cs` at `e5caa9b81d`; cited, not copied) makes one
extrusion per geometry object on its generic path and sets the body's
`ScaledArea` from each in turn, so on a body of several pieces the last
extrusion it creates would give `GrossAreaCalculator` its value. This tests
that on Revit's own export (the second argument): for each `IfcCovering` whose
`RevitClass` is `Ceiling`, with a `Qto_CoveringBaseQuantities.GrossArea` and a
Body of more than one item, the area of each item's extruded profile (the
extrusion's volume over its depth, a clipped item read through its first
operand), and which item's area equals `GrossArea` to 1e-3 relative: its rank
by STEP id (the order the export created them) and its place in the Items list
as written. Revit writes a layered ceiling with one item per layer, each of its
piece's area, so items of one area are taken as one piece, and a ceiling whose
items all have one area is not in pieces.

Revit's export carries no `RevitClass`: a covering is a ceiling where rvt-rs's
export (the first argument) gives the same Tag `RevitClass` `Ceiling`, or, for
a Tag rvt-rs does not write, where Revit's `PredefinedType` is `CEILING`.
rvt-rs's Body item count for the Tag is printed too, so the lines say whether
rvt-rs writes the ceiling in pieces.

Prints one `piece` line per ceiling, naming its Tag (so a held-out model's
must never be kept), and the counts:

    ceilings in pieces with GrossArea N: GrossArea one piece's M; holding the highest id A, lowest id B;
        last listed C, first listed D; largest E, smallest F

Areas in square feet.

Usage:

    python3 tools/re/ceiling_piece_order_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>
"""

import os
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element as ue
import ifcopenshell.util.unit
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import qtos_vs_ifc  # noqa: E402

FOOT = 0.3048
SET = "Qto_CoveringBaseQuantities"
TOL = 1e-3


def body(product):
    if not product.Representation:
        return None
    return next(
        (r for r in product.Representation.Representations if r.RepresentationIdentifier == "Body"),
        None,
    )


def extrusion(item):
    """The `IfcExtrudedAreaSolid` an item is, or is clipped from."""
    while item is not None and item.is_a("IfcBooleanResult"):
        item = item.FirstOperand
    return item if item is not None and item.is_a("IfcExtrudedAreaSolid") else None


def volume(geometry):
    v = np.asarray(geometry.verts, dtype=float).reshape(-1, 3)
    t = np.asarray(geometry.faces, dtype=int).reshape(-1, 3)
    if not len(t):
        return None
    a, b, c = v[t[:, 0]], v[t[:, 1]], v[t[:, 2]]
    return abs(float(np.einsum("ij,ij->i", a, np.cross(b, c)).sum()) / 6.0)


def profile_area(settings, solid, length_scale):
    """The profile area of an extrusion in square metres, or None."""
    try:
        geometry = ifcopenshell.geom.create_shape(settings, solid)
    except Exception:
        return None
    vol = volume(geometry)
    direction = np.asarray(solid.ExtrudedDirection.DirectionRatios, dtype=float)
    norm = np.linalg.norm(direction)
    depth = float(solid.Depth) * length_scale
    if vol is None or norm == 0 or depth <= 0 or abs(direction[2]) / norm < 1e-9:
        return None
    return vol / (depth * abs(direction[2]) / norm)


def ours_by_tag(path):
    """rvt-rs's coverings by Tag: (`RevitClass`, Body item count)."""
    f = qtos_vs_ifc.open_ifc(path)
    out = {}
    for product in f.by_type("IfcCovering"):
        if not product.Tag:
            continue
        psets = ue.get_psets(product)
        cls = next((str(p["RevitClass"]) for p in psets.values() if "RevitClass" in p), None)
        rep = body(product)
        out.setdefault(str(product.Tag), (cls, len(rep.Items) if rep is not None else 0))
    return out


def main():
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    ours = ours_by_tag(sys.argv[1])
    path = sys.argv[2]
    f = qtos_vs_ifc.open_ifc(path)
    quantities = qtos_vs_ifc.quantities(path)[0]
    length_scale = ifcopenshell.util.unit.calculate_unit_scale(f, "LENGTHUNIT")
    settings = ifcopenshell.geom.settings()
    counts = dict.fromkeys(
        ["pieces", "one item", "highest id", "lowest id", "last listed", "first listed", "largest", "smallest"], 0
    )
    unread = 0
    for product in f.by_type("IfcCovering"):
        if not product.Tag:
            continue
        tag = str(product.Tag)
        if tag in ours:
            if ours[tag][0] != "Ceiling":
                continue
        elif product.PredefinedType != "CEILING":
            continue
        rep = body(product)
        if rep is None or len(rep.Items) < 2:
            continue
        gross = quantities.get((f"tag:{tag}", SET, "GrossArea"))
        if gross is None or gross[1] is None:
            continue
        gross = gross[1] / FOOT**2
        items = list(rep.Items)
        areas = []
        for item in items:
            solid = extrusion(item)
            area = profile_area(settings, solid, length_scale) if solid is not None else None
            areas.append(None if area is None else area / FOOT**2)
        if any(a is None for a in areas):
            unread += 1
            print(f"piece {tag} items={len(items)} unread")
            continue
        # A layered ceiling has one item per layer, each of its piece's area:
        # items of one area are one piece.
        pieces = []
        for i, a in enumerate(areas):
            home = next((p for p in pieces if abs(areas[p[0]] - a) <= TOL * max(a, 1e-9)), None)
            if home is None:
                pieces.append([i])
            else:
                home.append(i)
        if len(pieces) < 2:
            continue
        counts["pieces"] += 1
        matches = [p for p in pieces if abs(areas[p[0]] - gross) <= TOL * max(gross, 1e-9)]
        by_id = sorted(range(len(items)), key=lambda i: items[i].id())
        cut = any(item.is_a("IfcBooleanResult") for item in items)
        if len(matches) == 1:
            piece = matches[0]
            sizes = [areas[p[0]] for p in pieces]
            counts["one item"] += 1
            counts["highest id"] += by_id[-1] in piece
            counts["lowest id"] += by_id[0] in piece
            counts["last listed"] += len(items) - 1 in piece
            counts["first listed"] += 0 in piece
            counts["largest"] += areas[piece[0]] == max(sizes)
            counts["smallest"] += areas[piece[0]] == min(sizes)
            ranks = ",".join(str(by_id.index(i) + 1) for i in piece)
            listed = ",".join(str(i + 1) for i in piece)
            where = f"pieces={len(pieces)} id_rank={ranks}/{len(items)} listed={listed}/{len(items)}"
        else:
            where = f"pieces={len(pieces)} matches={len(matches)}"
        shown = ",".join(f"{a:.4f}" for a in areas)
        print(
            f"piece {tag} items={len(items)} cut={int(cut)} gross={gross:.4f} {where} "
            f"areas={shown} ids={','.join(str(i.id()) for i in items)} rvt_rs_items={ours.get(tag, (None, 0))[1]}"
        )
    print(
        f"ceilings in pieces with GrossArea {counts['pieces']}: GrossArea one piece's {counts['one item']}; "
        f"holding the highest id {counts['highest id']}, lowest id {counts['lowest id']}; "
        f"last listed {counts['last listed']}, first listed {counts['first listed']}; "
        f"largest {counts['largest']}, smallest {counts['smallest']}; unread {unread}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())

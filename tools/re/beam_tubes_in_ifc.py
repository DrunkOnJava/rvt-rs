#!/usr/bin/env python3
"""Print how Revit's IFC4 export draws a beam whose profile has voids, beside what
rvt-rs writes for the same beam.

Research tool for #1698. The BIM4LCA office's 8 `Putkipalkki - suorakaide:PA-1` beams
are hollow rectangular tubes. Revit's export draws each as an `IfcExtrudedAreaSolid`
over an `IfcArbitraryProfileDefWithVoids` whose outer curve and void are
`IfcIndexedPolyCurve`s (lines and arcs); rvt-rs draws a solid box. To write Revit's own
form the writer needs what this prints: each curve's points and segment indices in the
order Revit writes them, the extrusion's placement and direction, and how its depth
relates to the beam's `Length`.

For every `IfcBeam` of Revit's export whose body holds such a profile (directly or through
an `IfcMappedItem`) it prints, in millimetres (the file's length unit scaled):

    beam <Tag> placement <4x4 world matrix rows>
      mapped <origin, target scale> | direct
      extrusion position <location, axis, ref direction> direction <x y z> depth <mm>
      Length <mm> (Qto_BeamBaseQuantities) | Axis curve length <mm>
      outer <n> points, segments <LINE(i,j) | ARC(i,j,k) ...>
        <x y> ...
      void <n> points, segments ...
        <x y> ...

then how many beams share one signature, and, for the same Tags in rvt-rs's export, the
kind of solid it writes. Counts and geometry only; no names.

Usage:

    python3 tools/re/beam_tubes_in_ifc.py <rvt-rs.ifc> <revit-export.ifc> [--list N]

`--list N` prints the full detail of the first N beams (default 2). Needs IfcOpenShell
(the version tools/ci/requirements-ifc.txt pins) and NumPy.
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.util.placement
import ifcopenshell.util.unit
import numpy as np


def mm(model):
    """Millimetres per file length unit."""
    return ifcopenshell.util.unit.calculate_unit_scale(model) * 1000.0


def body_solids(beam):
    """(solid, mapped item or None) for each IfcExtrudedAreaSolid of the beam's body."""
    out = []
    rep = beam.Representation
    if rep is None:
        return out
    for r in rep.Representations:
        if r.RepresentationIdentifier != "Body":
            continue
        for item in r.Items:
            if item.is_a("IfcMappedItem"):
                for inner in item.MappingSource.MappedRepresentation.Items:
                    if inner.is_a("IfcExtrudedAreaSolid"):
                        out.append((inner, item))
            elif item.is_a("IfcExtrudedAreaSolid"):
                out.append((item, None))
    return out


def curve_detail(curve, scale):
    """(points in mm, segment text) of an IfcIndexedPolyCurve, or (None, kind) of another."""
    if not curve.is_a("IfcIndexedPolyCurve"):
        return None, curve.is_a()
    points = [tuple(round(v * scale, 6) for v in p) for p in curve.Points.CoordList]
    segments = []
    for seg in curve.Segments or ():
        indices = ",".join(str(i) for i in seg.wrappedValue)
        segments.append(("ARC(" if seg.is_a("IfcArcIndex") else "LINE(") + indices + ")")
    if not segments:
        segments = ["(all points joined by lines)"]
    return points, " ".join(segments)


def axis_text(placement, scale):
    loc = tuple(round(v * scale, 6) for v in placement.Location.Coordinates)
    axis = tuple(round(v, 9) for v in placement.Axis.DirectionRatios) if placement.Axis else None
    ref = tuple(round(v, 9) for v in placement.RefDirection.DirectionRatios) if placement.RefDirection else None
    return f"location {loc} axis {axis} ref {ref}"


def revit_tubes(model):
    scale = mm(model)
    out = {}
    for beam in model.by_type("IfcBeam"):
        for solid, mapped in body_solids(beam):
            area = solid.SweptArea
            if not area.is_a("IfcArbitraryProfileDefWithVoids"):
                continue
            outer, outer_segments = curve_detail(area.OuterCurve, scale)
            voids = [curve_detail(c, scale) for c in area.InnerCurves]
            quantity = None
            for rel in beam.IsDefinedBy or ():
                definition = getattr(rel, "RelatingPropertyDefinition", None)
                if definition is not None and definition.is_a("IfcElementQuantity"):
                    for q in definition.Quantities:
                        if q.Name == "Length":
                            quantity = q.LengthValue * scale
            out[str(beam.Tag)] = {
                "beam": beam,
                "solid": solid,
                "mapped": mapped,
                "outer": outer,
                "outer_segments": outer_segments,
                "voids": voids,
                "length": quantity,
                "scale": scale,
            }
    return out


def signature(info):
    solid = info["solid"]
    return (
        tuple(info["outer"] or ()),
        info["outer_segments"],
        tuple((tuple(p or ()), s) for p, s in info["voids"]),
        axis_text(solid.Position, info["scale"]),
        tuple(round(v, 9) for v in solid.ExtrudedDirection.DirectionRatios),
        round(solid.Depth * info["scale"], 6),
    )


def print_detail(tag, info):
    beam, solid, mapped, scale = info["beam"], info["solid"], info["mapped"], info["scale"]
    matrix = ifcopenshell.util.placement.get_local_placement(beam.ObjectPlacement)
    print(f"beam {tag} placement")
    for row in matrix:
        print("   ", " ".join(f"{v:.9f}" for v in row))
    if mapped is None:
        print("  direct")
    else:
        origin = mapped.MappingSource.MappingOrigin
        target = mapped.MappingTarget
        print(f"  mapped origin {axis_text(origin, scale)} target scale {getattr(target, 'Scale', None)}")
    print(
        f"  extrusion position {axis_text(solid.Position, scale)} "
        f"direction {tuple(round(v, 9) for v in solid.ExtrudedDirection.DirectionRatios)} "
        f"depth {solid.Depth * scale:.6f}"
    )
    print(f"  Length {info['length']} mm (Qto_BeamBaseQuantities)")
    print(f"  outer {len(info['outer'] or [])} points, segments {info['outer_segments']}")
    for x, y in info["outer"] or []:
        print(f"    {x:.6f} {y:.6f}")
    for index, (points, segments) in enumerate(info["voids"], 1):
        print(f"  void {index}: {len(points or [])} points, segments {segments}")
        for x, y in points or []:
            print(f"    {x:.6f} {y:.6f}")


def ours_kinds(model, tags):
    """What rvt-rs writes for each tag: the solid's kind and its profile's."""
    kinds = collections.Counter()
    for beam in model.by_type("IfcBeam"):
        if str(beam.Tag) not in tags:
            continue
        found = []
        for solid, _ in body_solids(beam):
            found.append(f"{solid.is_a()} over {solid.SweptArea.is_a()}")
        if not found and beam.Representation is not None:
            for r in beam.Representation.Representations:
                if r.RepresentationIdentifier == "Body":
                    found.extend(i.is_a() for i in r.Items)
        kinds["; ".join(found) or "no body"] += 1
    return kinds


def main():
    args = sys.argv[1:]
    listing = 2
    if "--list" in args:
        i = args.index("--list")
        listing = int(args[i + 1])
        del args[i : i + 2]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    ours, revit = ifcopenshell.open(args[0]), ifcopenshell.open(args[1])
    tubes = revit_tubes(revit)
    beams = len(revit.by_type("IfcBeam"))
    print(f"beams in Revit's export: {beams}; with a profile with voids: {len(tubes)}")
    if not tubes:
        return 0
    signatures = collections.Counter(signature(info) for info in tubes.values())
    print(f"distinct signatures (profile, extrusion position, direction, depth): {len(signatures)}")
    shared = collections.Counter(signature(info)[:5] for info in tubes.values())
    print(f"distinct profiles and placements of the extrusion, depth apart: {len(shared)}")
    depths = collections.Counter(round(info["solid"].Depth * info["scale"], 3) for info in tubes.values())
    print("depths (mm): " + ", ".join(f"{d} x{n}" for d, n in sorted(depths.items())))
    lengths = [
        (info["solid"].Depth * info["scale"] - info["length"]) if info["length"] is not None else None
        for info in tubes.values()
    ]
    print(
        "depth minus Qto Length (mm): "
        + ", ".join(f"{v:.6f}" if v is not None else "-" for v in lengths)
    )
    for tag, info in list(sorted(tubes.items()))[:listing]:
        print_detail(tag, info)
    print("what rvt-rs writes for the same Tags:")
    for kind, count in ours_kinds(ours, set(tubes)).most_common():
        print(f"  {count:4}  {kind}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

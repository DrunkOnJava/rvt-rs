#!/usr/bin/env python3
"""Per beam, Revit's `Qto_BeamBaseQuantities` against its own body, and what rvt-rs writes (#1693, RE-423).

Pairs each `IfcBeam` of the two files by `Tag`. For each beam it reads `Length`, `CrossSectionArea`,
`OuterSurfaceArea`, `GrossSurfaceArea`, `NetSurfaceArea`, `GrossVolume` and `NetVolume` in each file's own units,
the kind of body Revit writes (an extrusion and its profile, or a mesh), and the body's mesh. A mesh is a prism
along an axis when all its planes but at most two (its ends) are parallel to it (`mullion_profiles_vs_ifc`'s
reading). Revit's extrusion need not run along the beam: a beam modelled as its side profile extruded by its
width has a `Depth` of the width, not of its `Length`. So the axis taken is the one whose extent is nearest the
`Depth` of Revit's extrusion (or, with no extrusion, the beam's `Length`), and the scorer says whether the
extrusion runs along the `Length`. From the axis it reads the extent, the area of the faces along it (lateral),
of the end faces square to it (flat), of those between (mitre), and the volume, and says which sum of them each
of Revit's quantities equals, by whether the extrusion runs along the `Length` and the kind of ends it has.

Then, for rvt-rs's beam, which of the quantities it writes and whether they equal Revit's, the body source it
carries, and whether its body is Revit's (the same extent, area and volume along the same axis): a beam whose
body is Revit's takes the sums above from its own mesh.

Revit's export in IFC2X3, which has no `Qto_` sets, is not scored.

Usage:

    python3 tools/re/beam_quantities_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>

Needs IfcOpenShell and numpy.
"""

import collections
import os
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import numpy as np  # noqa: E402
from mullion_profiles_vs_ifc import (  # noqa: E402
    AXIS_SINE,
    PARALLEL_COSINE,
    analyse,
    close,
    facts,
    fmt,
    mesh,
    planes_of,
    prism_axis,
    scales,
)
from saved_meshes_vs_ifc import body_sources  # noqa: E402

QUANTITIES = (
    "Length",
    "CrossSectionArea",
    "OuterSurfaceArea",
    "GrossSurfaceArea",
    "NetSurfaceArea",
    "GrossVolume",
    "NetVolume",
)
UNIT = {"Length": 0, "GrossVolume": 2, "NetVolume": 2}  # the index into scales(); the rest are areas (1)
LISTED = 8


def beam_values(beam, scale):
    """Each quantity of the beam's `Qto_BeamBaseQuantities` as metres, square or cubic metres, or None."""
    base = ifcopenshell.util.element.get_psets(beam).get("Qto_BeamBaseQuantities", {})
    out = {}
    for name in QUANTITIES:
        value = base.get(name)
        out[name] = None if value is None else float(value) * scale[UNIT.get(name, 1)]
    return out


def body_kind(beam):
    """What the beam's Body representation is made of: 'IfcExtrudedAreaSolid(IShapeProfileDef)', 'IfcFacetedBrep'."""
    kinds = []

    def items(representation, depth):
        for item in representation.Items:
            if item.is_a("IfcMappedItem") and depth < 3:
                items(item.MappingSource.MappedRepresentation, depth + 1)
                continue
            kind = item.is_a()
            if item.is_a("IfcExtrudedAreaSolid"):
                kind += f"({item.SweptArea.is_a()[3:-3]})"
            kinds.append(kind)

    for representation in beam.Representation.Representations if beam.Representation else ():
        if representation.RepresentationIdentifier == "Body":
            items(representation, 0)
    return "+".join(sorted(set(kinds))) or "none"


def curve_points(curve):
    """The 2D points of a polyline or an indexed poly curve of lines, or None (an arc is not read)."""
    if curve.is_a("IfcPolyline"):
        return [tuple(p.Coordinates[:2]) for p in curve.Points]
    if curve.is_a("IfcIndexedPolyCurve"):
        if curve.Segments and any(segment.is_a("IfcArcIndex") for segment in curve.Segments):
            return None
        return [tuple(p[:2]) for p in curve.Points.CoordList]
    return None


def polygon(points):
    """The perimeter and the area of a closed polygon given by its points."""
    if points is None or len(points) < 3:
        return None
    ring = points if points[0] != points[-1] else points[:-1]
    perimeter = sum(((a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2) ** 0.5 for a, b in zip(ring, ring[1:] + ring[:1]))
    area = abs(sum(a[0] * b[1] - b[0] * a[1] for a, b in zip(ring, ring[1:] + ring[:1]))) / 2
    return perimeter, area


def profile_of(solid, factor):
    """The depth, the profile's area, its outer perimeter and the perimeters of its voids, in metres, of an extrusion
    (None for what is not an extrusion of a rectangle or of a polygon, and for a profile with an arc)."""
    depth = float(solid.Depth) * factor
    area = solid.SweptArea
    if area.is_a("IfcRectangleProfileDef"):
        x, y = float(area.XDim) * factor, float(area.YDim) * factor
        return {"depth": depth, "area": x * y, "outer": 2 * (x + y), "inner": 0.0}
    if area.is_a("IfcArbitraryClosedProfileDef"):
        outer = polygon(curve_points(area.OuterCurve))
        if outer is None:
            return {"depth": depth}
        inner = [polygon(curve_points(c)) for c in (area.InnerCurves if area.is_a("IfcArbitraryProfileDefWithVoids") else ())]
        if any(v is None for v in inner):
            return {"depth": depth}
        return {
            "depth": depth,
            "area": (outer[1] - sum(v[1] for v in inner)) * factor**2,
            "outer": outer[0] * factor,
            "inner": sum(v[0] for v in inner) * factor,
        }
    return {"depth": depth}


def extrusion(beam, scale):
    """What the first IfcExtrudedAreaSolid of the beam's Body is (see `profile_of`), or None."""
    found = []

    def items(representation, depth):
        for item in representation.Items:
            if item.is_a("IfcMappedItem") and depth < 3:
                items(item.MappingSource.MappedRepresentation, depth + 1)
            elif item.is_a("IfcExtrudedAreaSolid"):
                found.append(item)

    for representation in beam.Representation.Representations if beam.Representation else ():
        if representation.RepresentationIdentifier == "Body":
            items(representation, 0)
    return profile_of(found[0], scale[0]) if found else None


def sums(m, quantity):
    """What a prism could give for `quantity`, by name: the extent, the volume, or a sum of its faces."""
    if quantity == "Length":
        return {"extent": m["extent"]}
    if quantity in ("GrossVolume", "NetVolume"):
        return {"volume": m["volume"]}
    flats = m["ends"].count("flat")
    one = m["flat"] / flats if flats else 0.0
    return {
        "lateral": m["lateral"],
        "lateral + flat ends": m["lateral"] + m["flat"],
        "lateral + mitre": m["lateral"] + m["mitre"],
        "all faces": m["area"],
        "one flat end": one,
        "lateral + one flat end": m["lateral"] + one,
        "all faces less one flat end": m["area"] - one,
    }


def profile_sums(profile, quantity):
    """What the extrusion's own profile gives for `quantity`: the area, the outer perimeter times the depth, and the
    outer or all perimeters times the depth plus the two ends."""
    if not profile or "area" not in profile:
        return {}
    depth, area, outer, inner = profile["depth"], profile["area"], profile["outer"], profile["inner"]
    if quantity == "CrossSectionArea":
        return {"profile area": area}
    if quantity == "OuterSurfaceArea":
        return {"outer perimeter x depth": outer * depth}
    if quantity == "GrossSurfaceArea":
        return {"outer perimeter x depth + 2 areas": outer * depth + 2 * area}
    if quantity == "NetSurfaceArea":
        return {"all perimeters x depth + 2 areas": (outer + inner) * depth + 2 * area}
    if quantity in ("GrossVolume", "NetVolume"):
        return {"profile area x depth": area * depth}
    return {}


def equals(m, quantity, value, profile=None):
    """The names of the sums of the prism and of the extrusion's profile that equal `value`; one that gives what an
    earlier one did is in brackets."""
    if value is None:
        return ()
    candidates = dict(profile_sums(profile, quantity))
    if m is not None:
        candidates.update(sums(m, quantity))
    seen, found = [], []
    for name, total in candidates.items():
        if not close(total, value):
            continue
        found.append(f"({name})" if any(close(total, other) for other in seen) else name)
        seen.append(total)
    return tuple(found)


def ours_has_length(values):
    return values["Length"] is not None


def prism_axes(triangles):
    """Every axis the body is a prism along (at most two of its planes are not parallel to it), longest extent
    first, as (axis, extent, kinds of its two ends)."""
    kept, normals, _, offsets = facts(triangles)
    planes = planes_of(normals, offsets)
    directions = []
    for i in range(len(planes)):
        for j in range(i + 1, len(planes)):
            c = np.cross(planes[i][0], planes[j][0])
            norm = np.linalg.norm(c)
            if norm < AXIS_SINE:
                continue
            c = c / norm
            if not any(abs(float(c @ k)) > 1 - 1e-6 for k in directions):
                directions.append(c)
    points = kept.reshape(-1, 3)
    out = []
    for axis in directions:
        ends = sorted("flat" if abs(float(p[0] @ axis)) > 0.9999 else "mitre" for p in planes if abs(float(p[0] @ axis)) > PARALLEL_COSINE)
        if len(ends) <= 2:
            out.append((axis, float(np.ptp(points @ axis)), tuple(ends)))
    return sorted(out, key=lambda a: -a[1])


def long_extent(triangles):
    """The extent of the body along its long axis (the first principal axis of its vertices), in metres."""
    points = triangles.reshape(-1, 3)
    centred = points - points.mean(axis=0)
    _, _, vt = np.linalg.svd(centred, full_matrices=False)
    return float(np.ptp(points @ vt[0]))


def axis_rule(axes, depth):
    """Where Revit's extrusion axis (the one whose extent is `depth`) stands among the prism axes: its rank by
    extent, how many there are, and whether the longest with two flat ends is it."""
    if depth is None or not axes:
        return None
    rank = min(range(len(axes)), key=lambda i: abs(axes[i][1] - depth))
    flat = [i for i, a in enumerate(axes) if a[2] == ("flat", "flat")]
    return rank + 1, len(axes), "the longest flat-ended prism axis" if flat and flat[0] == rank else (
        "not the longest flat-ended" if flat else "no flat-ended axis")


def read(settings, beam, scale, hint=None):
    """The beam's quantities, its extrusion depth, and its body analysed along the prism axis nearest `hint`,
    or its extrusion's depth, or its own `Length`."""
    values = beam_values(beam, scale)
    profile = extrusion(beam, scale)
    depth = profile["depth"] if profile else None
    triangles = mesh(settings, beam)
    reach = hint if hint is not None else (depth if depth is not None else values["Length"])
    axis = prism_axis(triangles, reach) if triangles is not None and reach else None
    return values, profile, (analyse(triangles, axis) if axis is not None else None)


def main():
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    ours_model, revit_model = ifcopenshell.open(sys.argv[1]), ifcopenshell.open(sys.argv[2])
    if revit_model.schema == "IFC2X3":
        print("Revit's export is IFC2X3, which has no Qto_ sets: not scored")
        return 0
    ours_scale, revit_scale = scales(ours_model), scales(revit_model)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    ours = {str(b.Tag): b for b in ours_model.by_type("IfcBeam") if b.Tag}
    revit = {str(b.Tag): b for b in revit_model.by_type("IfcBeam") if b.Tag}
    both = sorted(set(ours) & set(revit), key=lambda t: (len(t), t))
    print(f"beams: rvt-rs {len(ours)}, Revit {len(revit)}, paired by Tag {len(both)}")
    if not both:
        return 0
    kinds, written, rules, bodies = collections.Counter(), collections.Counter(), collections.Counter(), collections.Counter()
    choice, lengths, ratios = collections.Counter(), collections.Counter(), []
    odd = []
    for tag in both:
        r, profile, rb = read(settings, revit[tag], revit_scale)
        depth = profile["depth"] if profile else None
        o, _, ob = read(settings, ours[tag], ours_scale, depth if depth is not None else r["Length"])
        run = "no extrusion" if depth is None else ("along the Length" if close(depth, r["Length"]) else "across the Length")
        kinds[(body_kind(revit[tag]), run)] += 1
        for name in QUANTITIES:
            if r[name] is None:
                continue
            state = "absent" if o[name] is None else ("equal" if close(o[name], r[name]) else "different")
            written[(name, state)] += 1
        ends = rb["ends"] if rb else None
        triangles = mesh(settings, revit[tag])
        axes = prism_axes(triangles) if triangles is not None else []
        choice[(run, axis_rule(axes, depth))] += 1
        if triangles is not None and r["Length"]:
            long = long_extent(triangles)
            same = close(long, r["Length"])
            lengths[(run, "Length is the body's extent along its long axis" if same else "Length differs from it", "rvt-rs writes it" if ours_has_length(o) else "rvt-rs writes none")] += 1
            if not same:
                ratios.append(r["Length"] / long)
        for name in QUANTITIES:
            if r[name] is None:
                continue
            rules[(name, run, ends, equals(rb, name, r[name], profile) or ("none" if rb else "no prism axis"))] += 1
        same = ob is not None and rb is not None and close(ob["extent"], rb["extent"]) and close(ob["area"], rb["area"]) and close(ob["volume"], rb["volume"])
        src = body_sources(ours[tag])
        bodies[(run, ends, src, "our body is Revit's" if same else "our body differs")] += 1
        missing = [name for name in QUANTITIES[1:] if r[name] is not None and not equals(rb, name, r[name], profile)]
        if missing and len(odd) < LISTED:
            odd.append((tag, revit[tag].Name, src, r, rb, o, ob, missing, depth))
    print("Revit's body of each beam, and whether its extrusion runs along the Length:")
    for kind, count in kinds.most_common():
        print(f"  {count:5}  {kind}")
    print("what rvt-rs writes of each quantity Revit writes (quantity, state):")
    for key, count in sorted(written.items()):
        print(f"  {count:5}  {key}")
    print("Revit's quantity as a sum of its own body's faces (quantity, extrusion, ends of Revit's body, the sums that equal it):")
    for key, count in sorted(rules.items(), key=lambda kv: (QUANTITIES.index(kv[0][0]), -kv[1])):
        print(f"  {count:5}  {key}")
    print("Revit's extrusion axis among the axes its body is a prism along (extrusion, (rank by extent, how many axes, the rule)):")
    for key, count in choice.most_common():
        print(f"  {count:5}  {key}")
    print("Revit's Length against the extent of its own body along its long axis (extrusion, which, whether rvt-rs writes a Length):")
    for key, count in lengths.most_common():
        print(f"  {count:5}  {key}")
    if ratios:
        ratios.sort()
        print(f"  where it differs, Revit's Length over the long extent: min {ratios[0]:.6g}, median {ratios[len(ratios) // 2]:.6g}, max {ratios[-1]:.6g}")
    print("rvt-rs's body against Revit's (extrusion, ends of Revit's body, rvt-rs's body source):")
    for key, count in bodies.most_common():
        print(f"  {count:5}  {key}")
    print(f"the first {len(odd)} beams with a quantity that is no sum of Revit's own faces:")
    for tag, name, src, r, rb, o, ob, missing, depth in odd:
        print(f"  beam {tag} {name!r} src {src} extrusion depth {fmt(depth)} no sum for {','.join(missing)}")
        print("      Revit " + " | ".join(f"{q} {fmt(r[q])}" for q in QUANTITIES))
        print("      ours  " + " | ".join(f"{q} {fmt(o[q])}" for q in QUANTITIES))
        for label, m in (("Revit", rb), ("ours", ob)):
            if m:
                print(
                    f"      {label} body: ends {'+'.join(m['ends'])} extent {m['extent']:.6g} area {m['area']:.6g} lateral {m['lateral']:.6g} "
                    f"flat {m['flat']:.6g} mitre {m['mitre']:.6g} volume {m['volume']:.6g}"
                )
            else:
                print(f"      {label} body: no prism axis near the Length")
    return 0


if __name__ == "__main__":
    sys.exit(main())

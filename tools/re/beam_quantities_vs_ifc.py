#!/usr/bin/env python3
"""Per beam, Revit's `Qto_BeamBaseQuantities` against its own body, and what rvt-rs writes (#1693, RE-423).

Pairs each `IfcBeam` of the two files by `Tag`. For each beam it reads `Length`, `CrossSectionArea`,
`OuterSurfaceArea`, `GrossSurfaceArea`, `NetSurfaceArea`, `GrossVolume` and `NetVolume` in each file's own units,
the kind of body Revit writes (an extrusion and its profile, or a mesh), and the body's mesh. A mesh is a prism
along an axis when all its planes but at most two (its ends) are parallel to it (`mullion_profiles_vs_ifc`'s
reading, taking the axis whose extent is nearest the beam's own `Length`): from it the scorer reads the extent,
the area of the faces along the axis (lateral), of the end faces square to it (flat), of those between (mitre),
and the volume, and says which sum of them each of Revit's quantities equals, by the kind of ends the body has.

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
from mullion_profiles_vs_ifc import analyse, close, fmt, mesh, prism_axis, scales  # noqa: E402
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


def equals(m, quantity, value):
    """The names of the sums of the prism that equal `value`; one that gives what an earlier one did is in brackets."""
    if m is None or value is None:
        return ()
    seen, found = [], []
    for name, total in sums(m, quantity).items():
        if not close(total, value):
            continue
        found.append(f"({name})" if any(close(total, other) for other in seen) else name)
        seen.append(total)
    return tuple(found)


def read(settings, beam, scale, hint=None):
    """The beam's quantities, and its body analysed along the prism axis nearest `hint` or its own `Length`."""
    values = beam_values(beam, scale)
    triangles = mesh(settings, beam)
    reach = hint if hint is not None else values["Length"]
    axis = prism_axis(triangles, reach) if triangles is not None and reach else None
    return values, (analyse(triangles, axis) if axis is not None else None)


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
    odd = []
    for tag in both:
        r, rb = read(settings, revit[tag], revit_scale)
        hint = r["Length"]
        o, ob = read(settings, ours[tag], ours_scale, hint)
        kinds[body_kind(revit[tag])] += 1
        for name in QUANTITIES:
            if r[name] is None:
                continue
            state = "absent" if o[name] is None else ("equal" if close(o[name], r[name]) else "different")
            written[(name, state)] += 1
        ends = rb["ends"] if rb else None
        for name in QUANTITIES:
            if r[name] is None:
                continue
            rules[(name, ends, equals(rb, name, r[name]) or ("none" if rb else "no prism axis"))] += 1
        same = ob is not None and rb is not None and close(ob["extent"], rb["extent"]) and close(ob["area"], rb["area"]) and close(ob["volume"], rb["volume"])
        src = body_sources(ours[tag])
        bodies[(ends, src, "our body is Revit's" if same else "our body differs")] += 1
        missing = [name for name in QUANTITIES[1:] if r[name] is not None and not equals(rb, name, r[name])]
        if missing and len(odd) < LISTED:
            odd.append((tag, revit[tag].Name, src, r, rb, o, ob, missing))
    print("Revit's body of each beam:")
    for kind, count in kinds.most_common():
        print(f"  {count:5}  {kind}")
    print("what rvt-rs writes of each quantity Revit writes (quantity, state):")
    for key, count in sorted(written.items()):
        print(f"  {count:5}  {key}")
    print("Revit's quantity as a sum of its own body's faces (quantity, ends of Revit's body, the sums that equal it):")
    for key, count in sorted(rules.items(), key=lambda kv: (QUANTITIES.index(kv[0][0]), -kv[1])):
        print(f"  {count:5}  {key}")
    print("rvt-rs's body against Revit's (ends of Revit's body, rvt-rs's body source):")
    for key, count in bodies.most_common():
        print(f"  {count:5}  {key}")
    print(f"the first {len(odd)} beams with a quantity that is no sum of Revit's own faces:")
    for tag, name, src, r, rb, o, ob, missing in odd:
        print(f"  beam {tag} {name!r} src {src} no sum for {','.join(missing)}")
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

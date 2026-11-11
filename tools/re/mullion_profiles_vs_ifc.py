#!/usr/bin/env python3
"""Per curtain mullion, rvt-rs's body and quantities against Revit's (#1046, #1636, RE-404, RE-418).

Pairs each `IfcMember` of PredefinedType MULLION of the two files by `Tag` and reads, for each side, its
`Length`, `NetVolume` and `NetSurfaceArea` (`Qto_MemberBaseQuantities`), the `Span` of its
`Pset_MemberCommon`, and the body's mesh. A mesh is a prism when all its planes but at most two (its ends)
are parallel to one axis; where several axes make it one (a box has three) the scorer takes the one whose
extent is nearest the side's own `Length`, and reads from it the extent, the area of the faces parallel to
the axis (lateral), of the end faces square to it (flat) and of those between (mitre), and the volume.

Three tallies say what a rule would change, by what the mullion's ends are:

  - RE-418: whether `Length` and `Span` taken from the mesh's extent along its axis equal Revit's, where the
    written ones do not and where they do;
  - RE-404: which sums of the faces equal Revit's `NetSurfaceArea` (lateral, plus the flat ends, plus the
    mitre ends), by the ends of our mesh, where the written value is Revit's and where it is not;
  - whether Revit's `Length` is the extent of its own body.

A few mullions of each kind of difference (by which quantities differ and the ends of the body) are printed, the rest counted.

Usage:

    python3 tools/re/mullion_profiles_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>

Needs IfcOpenShell and numpy.
"""

import collections
import os
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element
import ifcopenshell.util.unit
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from saved_meshes_vs_ifc import body_sources  # noqa: E402

LISTED = 6
TOLERANCE = 1e-5
# The thresholds of src/ifc/mesh_volume.rs's prism_extents, which this mirrors, in metres.
MIN_TWICE_AREA = 1.8e-12
PLANE_COSINE = 1e-5
PLANE_METRES = 2.1e-4
PARALLEL_COSINE = 1e-3
AXIS_SINE = 1e-2
# The extent along the axis a rule would take must lie this close to the written Length, so that a box's
# other axes are never taken for the mullion's own.
AXIS_REACH = 0.35


def is_mullion(member):
    return member.is_a("IfcMember") and getattr(member, "PredefinedType", None) == "MULLION" and member.Tag


def scales(model):
    """Metres per length unit, square metres per area unit and cubic metres per volume unit of a file: the
    three are declared apart (Revit's export has millimetres, square metres and cubic metres)."""
    out = []
    for unit in ("LENGTHUNIT", "AREAUNIT", "VOLUMEUNIT"):
        try:
            out.append(float(ifcopenshell.util.unit.calculate_unit_scale(model, unit)))
        except Exception:  # noqa: BLE001
            out.append(1.0)
    return out


def values(member, scale):
    """Length, NetVolume, NetSurfaceArea and Span as metres, cubic and square metres, or None."""
    out = {}
    length, area, volume = scale
    psets = ifcopenshell.util.element.get_psets(member)
    base = psets.get("Qto_MemberBaseQuantities", {})
    for name, factor in (("Length", length), ("NetVolume", volume), ("NetSurfaceArea", area)):
        value = base.get(name)
        out[name] = None if value is None else float(value) * factor
    span = psets.get("Pset_MemberCommon", {}).get("Span")
    out["Span"] = None if span is None else float(span) * length
    return out


def mesh(settings, member):
    """Distinct triangles of the member's body in model coordinates (metres), or None."""
    try:
        shape = ifcopenshell.geom.create_shape(settings, member)
    except Exception:  # noqa: BLE001
        return None
    v = np.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
    f = np.array(shape.geometry.faces, dtype=int).reshape(-1, 3)
    if not len(f):
        return None
    triangles = v[f]
    seen, distinct = set(), []
    for t in triangles:
        key = tuple(sorted(tuple(np.round(p, 6)) for p in t))
        if key not in seen:
            seen.add(key)
            distinct.append(t)
    return np.array(distinct)


def facts(triangles):
    """The triangles that have an area, with their unit normals, areas and plane offsets."""
    cross = np.cross(triangles[:, 1] - triangles[:, 0], triangles[:, 2] - triangles[:, 0])
    area = np.linalg.norm(cross, axis=1) / 2
    keep = 2 * area >= MIN_TWICE_AREA
    kept = triangles[keep]
    normals = cross[keep] / (2 * area[keep])[:, None]
    return kept, normals, area[keep], (normals * kept[:, 0]).sum(axis=1)


def planes_of(normals, offsets):
    """The distinct planes of a mesh as [normal, offset]: triangles on one plane agree to 0.2 mm."""
    planes = []
    for n, d in zip(normals, offsets):
        for p in planes:
            s = float(n @ p[0])
            if abs(s) > 1 - PLANE_COSINE and abs(d - np.sign(s) * p[1]) < PLANE_METRES:
                break
        else:
            planes.append([n, d])
    return planes


def prism_axis(triangles, hint):
    """The axis along which the body is a prism (at most two of its planes are not parallel to it), taking
    among those the one whose extent is nearest `hint`; None when there is none within AXIS_REACH of it."""
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
    best = None
    for axis in directions:
        if sum(1 for p in planes if abs(float(p[0] @ axis)) > PARALLEL_COSINE) > 2:
            continue
        extent = float(np.ptp(points @ axis))
        if best is None or abs(extent - hint) < abs(best[1] - hint):
            best = (axis, extent)
    if best is None or abs(best[1] - hint) > AXIS_REACH * hint:
        return None
    return best[0]


def analyse(triangles, axis):
    """The extent along the axis, the lateral, flat and mitre areas, the volume and the kind of each end."""
    kept, normals, area, offsets = facts(triangles)
    along = np.abs(normals @ axis)
    lateral, flat = along < PARALLEL_COSINE, along > 0.9999
    middle = ~lateral & ~flat
    ends = sorted("flat" if abs(float(p[0] @ axis)) > 0.9999 else "mitre" for p in planes_of(normals, offsets) if abs(float(p[0] @ axis)) > PARALLEL_COSINE)
    volume = abs(float(np.einsum("ij,ij->i", kept[:, 0], np.cross(kept[:, 1], kept[:, 2])).sum() / 6))
    return {
        "extent": float(np.ptp(kept.reshape(-1, 3) @ axis)),
        "area": float(area.sum()),
        "lateral": float(area[lateral].sum()),
        "flat": float(area[flat].sum()),
        "mitre": float(area[middle].sum()),
        "volume": volume,
        "ends": tuple(ends),
        "triangles": len(kept),
    }


def read(settings, member, scale):
    """The member's quantities, and its body analysed along the prism axis nearest its Length (or None)."""
    v = values(member, scale)
    triangles = mesh(settings, member)
    axis = prism_axis(triangles, v["Length"]) if triangles is not None and v["Length"] else None
    return v, (analyse(triangles, axis) if axis is not None else None)


def close(a, b):
    if a is None or b is None:
        return a is None and b is None
    return abs(a - b) <= max(TOLERANCE * abs(b), 1e-6)


def fmt(x):
    return "-" if x is None else f"{x:.6g}"


def word(equal):
    return "equal" if equal else "different"


def decompose(m, value):
    """The ways `value` is the body's lateral area plus a number of its mitre face and of its flat ends
    (each flat end is half the flat area of a body with two): as labels such as 'lateral + 1 flat end'."""
    if value is None:
        return ()
    flats = m["ends"].count("flat")
    one = m["flat"] / flats if flats else 0.0
    found = []
    for mitres in ((0, 1) if m["mitre"] > 1e-9 else (0,)):
        for count in range(flats + 1):
            total = m["lateral"] + mitres * m["mitre"] + count * one
            if close(total, value):
                found.append(f"lateral{' + mitre' if mitres else ''}{f' + {count} flat end' if count else ''}")
    return tuple(found)


def main():
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    ours_model, revit_model = ifcopenshell.open(sys.argv[1]), ifcopenshell.open(sys.argv[2])
    ours_scale, revit_scale = scales(ours_model), scales(revit_model)
    print(f"units, metres per length unit, m2 and m3 per area and volume unit: rvt-rs {ours_scale}, Revit {revit_scale}")
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    ours = {str(m.Tag): m for m in ours_model.by_type("IfcMember") if is_mullion(m)}
    revit = {str(m.Tag): m for m in revit_model.by_type("IfcMember") if is_mullion(m)}
    both = sorted(set(ours) & set(revit), key=lambda t: (len(t), t))
    print(f"mullions: rvt-rs {len(ours)}, Revit {len(revit)}, paired by Tag {len(both)}")
    if not both:
        return 0
    lengths, areas, own = collections.Counter(), collections.Counter(), collections.Counter()
    tally, rows, listed = collections.Counter(), [], collections.Counter()
    for tag in both:
        o, ma = read(settings, ours[tag], ours_scale)
        r, mb = read(settings, revit[tag], revit_scale)
        diff = tuple(name for name in ("Length", "NetVolume", "NetSurfaceArea", "Span") if not close(o[name], r[name]))
        tally[diff] += 1
        src = body_sources(ours[tag])
        if ma is None:
            lengths[("no prism axis", word(close(o["Length"], r["Length"]) and close(o["Span"], r["Span"])))] += 1
        else:
            before = word(close(o["Length"], r["Length"]) and close(o["Span"], r["Span"]))
            after = word(close(ma["extent"], r["Length"]) and close(ma["extent"], r["Span"]))
            lengths[(ma["ends"], before + " to " + after)] += 1
            written = "absent" if o["NetSurfaceArea"] is None else word(close(o["NetSurfaceArea"], r["NetSurfaceArea"]))
            areas[(ma["ends"], "written " + written, "Revit: " + " | ".join(decompose(ma, r["NetSurfaceArea"])), "ours: " + " | ".join(decompose(ma, o["NetSurfaceArea"])))] += 1
        if mb is not None:
            own[("Revit's Length is its body's extent" if close(r["Length"], mb["extent"]) else "not", ma is not None and close(ma["extent"], mb["extent"]) and close(ma["area"], mb["area"]))] += 1
        kind = (diff, ma["ends"] if ma else None)
        if listed[kind] < LISTED:
            listed[kind] += 1
            rows.append((tag, diff, src, o, r, ma, mb, revit[tag].Name))
    print("by what differs:", {", ".join(k) or "nothing": v for k, v in tally.most_common()})
    for tag, diff, src, o, r, ma, mb, name in rows:
        name = (name or "-").split(":")[1] if ":" in (name or "") else (name or "-")
        print(
            f"mullion {tag} {name!r} src {src} differs {','.join(diff) or '-'} | "
            + " | ".join(f"{k} ours {fmt(o[k])} revit {fmt(r[k])}" for k in ("Length", "Span", "NetVolume", "NetSurfaceArea"))
        )
        for label, m in (("ours", ma), ("revit", mb)):
            if m:
                print(
                    f"    {label} body: ends {'+'.join(m['ends'])} extent {m['extent']:.6g} area {m['area']:.6g} lateral {m['lateral']:.6g} "
                    f"flat {m['flat']:.6g} mitre {m['mitre']:.6g} volume {m['volume']:.6g}"
                )
            else:
                print(f"    {label} body: no prism axis near the Length")
    print("RE-418, Length and Span from the extent of our body along its axis (ends of our body, written to rule):")
    for key, count in lengths.most_common():
        print(f"  {count:6}  {key}")
    print("RE-404, Revit's NetSurfaceArea as our body's faces (ends of our body, the written one, what Revit's is, what ours is):")
    for key, count in areas.most_common():
        print(f"  {count:6}  {key}")
    print("Revit's own Length against its body (Length is the extent of its body, our body is the same shape and area):")
    for key, count in own.most_common():
        print(f"  {count:6}  {key}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

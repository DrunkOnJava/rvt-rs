#!/usr/bin/env python3
"""Per curtain mullion, rvt-rs's body and quantities against Revit's (#1046, #1636, RE-404, RE-418).

Pairs each `IfcMember` of PredefinedType MULLION of the two files by `Tag` and reads, for each side, its
`Length`, `NetVolume` and `NetSurfaceArea` (`Qto_MemberBaseQuantities`), the `Span` of its
`Pset_MemberCommon`, and the body's mesh: the extent along its principal axis, its total area, the area of
the faces parallel to the axis (lateral), of the end faces square to it (flat) and of those between
(mitre), from the distinct triangles of its representation. A mullion that differs from Revit's in any of
its three quantities is printed with all of it; the rest are counted and the first few printed for
contrast. The totals say, for the mullions that differ, whether Revit's `Length` is the mesh's extent on
either side, and what Revit's `NetSurfaceArea` is made of (the lateral area, with which end faces added).

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

CONTRAST = 12
TOLERANCE = 1e-5


def is_mullion(member):
    return member.is_a("IfcMember") and getattr(member, "PredefinedType", None) == "MULLION" and member.Tag


def values(member, scale):
    """Length, NetVolume, NetSurfaceArea and Span as metres, cubic and square metres, or None."""
    out = {}
    psets = ifcopenshell.util.element.get_psets(member)
    base = psets.get("Qto_MemberBaseQuantities", {})
    for name, factor in (("Length", scale), ("NetVolume", scale**3), ("NetSurfaceArea", scale**2)):
        value = base.get(name)
        out[name] = None if value is None else float(value) * factor
    span = psets.get("Pset_MemberCommon", {}).get("Span")
    out["Span"] = None if span is None else float(span) * scale
    return out


def mesh(settings, member):
    """Distinct triangles of the member's body in model coordinates, or None."""
    try:
        shape = ifcopenshell.geom.create_shape(settings, member)
    except Exception:  # noqa: BLE001
        return None
    v = np.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
    f = np.array(shape.geometry.faces, dtype=int).reshape(-1, 3)
    if not len(f):
        return None
    triangles = v[f]
    keys = {tuple(sorted(tuple(np.round(p, 6)) for p in t)) for t in triangles}
    if len(keys) < len(triangles):
        seen, distinct = set(), []
        for t in triangles:
            key = tuple(sorted(tuple(np.round(p, 6)) for p in t))
            if key not in seen:
                seen.add(key)
                distinct.append(t)
        triangles = np.array(distinct)
    return triangles


def analyse(triangles):
    """The prism's axis is perpendicular to the normals of its large faces (its lateral faces, which
    dwarf its ends): the smallest right singular vector of those normals."""
    cross = np.cross(triangles[:, 1] - triangles[:, 0], triangles[:, 2] - triangles[:, 0])
    area = np.linalg.norm(cross, axis=1) / 2
    keep = area > 1e-12
    normals = cross[keep] / (2 * area[keep])[:, None]
    weights = area[keep]
    large = weights >= 0.2 * weights.max()
    _, _, vt = np.linalg.svd(normals[large], full_matrices=False)
    axis = vt[-1]
    points = triangles.reshape(-1, 3)
    projection = points @ axis
    along = np.abs(normals @ axis)
    lateral = along < 1e-4
    flat = along > 0.9999
    middle = ~lateral & ~flat
    volume = abs(float(np.einsum("ij,ij->i", triangles[:, 0], np.cross(triangles[:, 1], triangles[:, 2])).sum() / 6))
    return {
        "extent": float(projection.max() - projection.min()),
        "area": float(weights.sum()),
        "lateral": float(weights[lateral].sum()),
        "flat": float(weights[flat].sum()),
        "mitre": float(weights[middle].sum()),
        "volume": volume,
        "triangles": len(triangles),
    }


def close(a, b):
    if a is None or b is None:
        return a is None and b is None
    return abs(a - b) <= max(TOLERANCE * abs(b), 1e-6)


def fmt(x):
    return "-" if x is None else f"{x:.6g}"


def main():
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    ours_model, revit_model = ifcopenshell.open(sys.argv[1]), ifcopenshell.open(sys.argv[2])
    ours_scale = ifcopenshell.util.unit.calculate_unit_scale(ours_model)
    revit_scale = ifcopenshell.util.unit.calculate_unit_scale(revit_model)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    ours = {str(m.Tag): m for m in ours_model.by_type("IfcMember") if is_mullion(m)}
    revit = {str(m.Tag): m for m in revit_model.by_type("IfcMember") if is_mullion(m)}
    both = sorted(set(ours) & set(revit), key=lambda t: (len(t), t))
    print(f"mullions: rvt-rs {len(ours)}, Revit {len(revit)}, paired by Tag {len(both)}")
    if not both:
        return 0
    differing, equal_listed, tally = [], 0, collections.Counter()
    for tag in both:
        o, r = values(ours[tag], ours_scale), values(revit[tag], revit_scale)
        diff = tuple(name for name in ("Length", "NetVolume", "NetSurfaceArea", "Span") if not close(o[name], r[name]))
        tally[diff] += 1
        if diff:
            differing.append((tag, diff, o, r))
        elif equal_listed < CONTRAST:
            differing.append((tag, diff, o, r))
            equal_listed += 1
    print("by what differs:", {", ".join(k) or "nothing": v for k, v in tally.most_common()})
    kinds = collections.Counter()
    for tag, diff, o, r in differing:
        a = mesh(settings, ours[tag])
        b = mesh(settings, revit[tag])
        ma = analyse(a) if a is not None else None
        mb = analyse(b) if b is not None else None
        src = body_sources(ours[tag])
        name = (revit[tag].Name or "-").split(":")[1] if ":" in (revit[tag].Name or "") else (revit[tag].Name or "-")
        print(
            f"mullion {tag} {name!r} src {src} differs {','.join(diff) or '-'} | "
            + " | ".join(f"{k} ours {fmt(o[k])} revit {fmt(r[k])}" for k in ("Length", "Span", "NetVolume", "NetSurfaceArea"))
        )
        for label, m in (("ours", ma), ("revit", mb)):
            if m:
                print(
                    f"    {label} mesh: extent {m['extent']:.6g} area {m['area']:.6g} lateral {m['lateral']:.6g} "
                    f"flat {m['flat']:.6g} mitre {m['mitre']:.6g} volume {m['volume']:.6g} triangles {m['triangles']}"
                )
        if diff and mb:
            length_is = "revit mesh extent" if close(r["Length"], mb["extent"]) else "-"
            if ma and close(r["Length"], ma["extent"]):
                length_is += " + our mesh extent"
            made = []
            for label, extra in (
                ("lateral", 0.0),
                ("lateral+flat", mb["flat"]),
                ("lateral+mitre", mb["mitre"]),
                ("lateral+flat+mitre", mb["flat"] + mb["mitre"]),
                ("lateral+half flat", mb["flat"] / 2),
            ):
                if r["NetSurfaceArea"] is not None and close(r["NetSurfaceArea"], mb["lateral"] + extra):
                    made.append(label)
            kinds[("Length" in diff, length_is, tuple(made))] += 1
    print("what Revit's Length and NetSurfaceArea are, for the mullions that differ (length differs, Length equals, NetSurfaceArea made of):")
    for key, count in kinds.most_common():
        print(f"  {count:5}  {key}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

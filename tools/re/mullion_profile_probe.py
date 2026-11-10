#!/usr/bin/env python3
"""Scratch scorer (never merged): Revit's body and quantities of the curtain mullions of one type, against rvt-rs's
box formula, to see what Revit's NetSurfaceArea and NetVolume are made of when they differ from it (RE-402, RE-404).

Usage: python3 tools/re/mullion_profile_probe.py <rvt-rs.ifc> <revit-export.ifc> [type substring, default yläprofiili]
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element
import ifcopenshell.util.unit
import numpy as np


def qtos(member):
    return ifcopenshell.util.element.get_psets(member, qtos_only=True).get("Qto_MemberBaseQuantities", {})


def mesh_stats(settings, member):
    try:
        shape = ifcopenshell.geom.create_shape(settings, member)
    except Exception as error:  # noqa: BLE001
        return {"error": str(error)[:80]}
    v = np.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
    f = np.array(shape.geometry.faces, dtype=int).reshape(-1, 3)
    a, b, c = v[f[:, 0]], v[f[:, 1]], v[f[:, 2]]
    area = float(np.linalg.norm(np.cross(b - a, c - a), axis=1).sum() / 2)
    volume = float(abs(np.einsum("ij,ij->i", a, np.cross(b, c)).sum() / 6))
    lo, hi = v.min(axis=0), v.max(axis=0)
    return {"area": area, "volume": volume, "extents": sorted(float(x) for x in (hi - lo)), "triangles": len(f), "vertices": len(np.unique(v.round(6), axis=0))}


def body_items(member):
    out = []
    if member.Representation:
        for rep in member.Representation.Representations:
            if rep.RepresentationIdentifier == "Body":
                for item in rep.Items:
                    kind = item.is_a()
                    if kind == "IfcMappedItem":
                        kind += ">" + ",".join(i.is_a() for i in item.MappingSource.MappedRepresentation.Items)
                    if item.is_a("IfcExtrudedAreaSolid"):
                        profile = item.SweptArea
                        kind += f"/{profile.is_a()}"
                        if profile.is_a("IfcRectangleProfileDef"):
                            kind += f" {profile.XDim}x{profile.YDim}"
                        kind += f" depth {item.Depth}"
                    out.append(kind)
    return out


def main(argv):
    ours = ifcopenshell.open(argv[1])
    revit = ifcopenshell.open(argv[2])
    needles = [argv[3]] if len(argv) > 3 else ["yläprofiil", "120 x 30", "150 x 50", "50 x 150"]
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    mine = {str(m.Tag): m for m in ours.by_type("IfcMember") if m.Tag}
    by_type = collections.Counter()
    for m in revit.by_type("IfcMember"):
        if not m.Tag or str(m.Tag) not in mine:
            continue
        q, oq = qtos(m), qtos(mine[str(m.Tag)])
        nsa, onsa = q.get("NetSurfaceArea"), oq.get("NetSurfaceArea")
        nv, onv = q.get("NetVolume"), oq.get("NetVolume")
        v1 = "no nsa" if nsa is None or onsa is None else ("nsa equal" if abs(nsa - onsa) <= max(1e-5 * abs(nsa), 1e-6) else "NSA DIFFERENT")
        v2 = "no nv" if nv is None or onv is None else ("nv equal" if abs(nv - onv) <= max(1e-5 * abs(nv), 1e-9) else "NV DIFFERENT")
        by_type[((m.ObjectType or "-")[:60], v1, v2)] += 1
    print("Revit ObjectType, NetSurfaceArea verdict, NetVolume verdict: mullions")
    for (name, v1, v2), n in sorted(by_type.items()):
        print(f"  {name} | {v1} | {v2}: {n}")
    for needle in needles:
        probe_one(ours, revit, mine, settings, needle)
    return 0


def probe_one(ours, revit, mine, settings, needle):
    rows = []
    for m in revit.by_type("IfcMember"):
        if not m.Tag or needle not in (m.ObjectType or "").replace("_", " ") and needle not in (m.Name or "").replace("_", " "):
            continue
        q, o = qtos(m), mine.get(str(m.Tag))
        oq = qtos(o) if o is not None else {}
        rows.append((str(m.Tag), m, q, o, oq))
    print(f"\n=== Revit IfcMembers whose name or ObjectType holds {needle!r}: {len(rows)}; unit scale of Revit's file {ifcopenshell.util.unit.calculate_unit_scale(revit)}, of ours {ifcopenshell.util.unit.calculate_unit_scale(ours)}")
    kinds = collections.Counter()
    shown = {"different": 0, "equal": 0}
    for tag, m, q, o, oq in rows:
        nsa, nv, length = q.get("NetSurfaceArea"), q.get("NetVolume"), q.get("Length")
        onsa, onv = oq.get("NetSurfaceArea"), oq.get("NetVolume")
        verdict = "no value" if nsa is None or onsa is None else ("equal" if abs(nsa - onsa) <= max(1e-5 * abs(nsa), 1e-6) else "different")
        kinds[(verdict, tuple(body_items(m)))] += 1
        if verdict in shown and shown[verdict] < (14 if verdict == "different" else 6):
            shown[verdict] += 1
            stats = mesh_stats(settings, m)
            print(f"tag {tag} {verdict}: ObjectType {m.ObjectType!r} Revit Length {length} NetVolume {nv} NetSurfaceArea {nsa} | ours NetVolume {onv} NetSurfaceArea {onsa}")
            print(f"    Revit Qto keys {sorted(q)} ; body {body_items(m)} ; mesh {stats}")
            if o is not None:
                print(f"    ours body {body_items(o)} ; mesh {mesh_stats(settings, o)}")
    print("verdict, Revit body items: mullions")
    for (verdict, items), n in sorted(kinds.items(), key=lambda kv: -kv[1]):
        print(f"  {verdict}: {list(items)}: {n}")


if __name__ == "__main__":
    sys.exit(main(sys.argv))

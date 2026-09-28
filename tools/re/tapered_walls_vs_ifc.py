#!/usr/bin/env python3
"""Score tapered walls in an `rvt-ifc` IFC against Revit's own IFC4 export.

Research tool for `reports/element-framing/RE-86-tapered-walls.md`. It is an
end-to-end check: the IFC comes from the built `rvt-ifc` binary, and the
reference is the body Revit's exporter wrote for the same wall, both meshed
by IfcOpenShell.

Every Revit wall with a straight two-point axis is cut by horizontal planes
at 2, 25, 50, 75 and 98 per cent of its height. Each cut is measured across
and along the axis. A wall is tapered where its cut is more than 0.05 ft
wider at the base than at the top; each face's lean is its change across
the wall between the 1 and 99 per cent cuts.

With the probe's output (`examples/probe_re86_tapered_walls.rs`), each wall
is joined to its orientation word and, through Revit's
`IfcRelDefinesByType`, to its type's face angles. The script reports:

- which walls Revit tapers, their word, and whether the face that leans
  does so by the type's angle;
- for rvt-rs's body of each such wall, the largest difference from Revit's
  cut across and along the wall at any of the five heights, and its
  `BodySource`.

Revit's bodies are moved into Revit's internal frame through the export's
site placement, which is where rvt-rs places its own.

Usage:

    python3 tools/re/tapered_walls_vs_ifc.py <probe.jsonl> <rvt-rs.ifc> <revit-export.ifc> [--list N]

Needs IfcOpenShell (tested with 0.8.5) and NumPy.
"""

import json
import math
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.placement
import numpy as np

FEET = 0.3048
HEIGHTS = (0.02, 0.25, 0.5, 0.75, 0.98)
TAPER_FEET = 0.05


def site_frame(f):
    """Shared metres (IfcOpenShell world) -> Revit internal feet."""
    site = f.by_type("IfcSite")[0]
    mat = ifcopenshell.util.placement.get_local_placement(site.ObjectPlacement)
    ox, oy, oz = mat[0][3], mat[1][3], mat[2][3]
    c, s = mat[0][0], mat[1][0]

    def points(v):
        dx, dy = v[:, 0] - ox, v[:, 1] - oy
        return np.stack(
            [(c * dx + s * dy) / FEET, (-s * dx + c * dy) / FEET, (v[:, 2] - oz) / FEET], axis=1
        )

    return points


def meshes(f, walls, frame):
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    out = {}
    for wall in walls:
        try:
            shape = ifcopenshell.geom.create_shape(settings, wall)
        except Exception:
            continue
        verts = np.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
        faces = np.array(shape.geometry.faces, dtype=int).reshape(-1, 3)
        if len(verts) and len(faces):
            out[wall.Tag] = (frame(verts), faces)
    return out


def cut(mesh, z, origin, along, across):
    """(min, max) across and (min, max) along of the mesh's section at z."""
    verts, faces = mesh
    tri = verts[faces]
    xs, ys = [], []
    for k in range(3):
        p, q = tri[:, k], tri[:, (k + 1) % 3]
        crossing = (p[:, 2] - z) * (q[:, 2] - z) < 0
        if not crossing.any():
            continue
        p, q = p[crossing], q[crossing]
        t = ((z - p[:, 2]) / (q[:, 2] - p[:, 2]))[:, None]
        hit = p[:, :2] + (q[:, :2] - p[:, :2]) * t - origin
        xs.extend(hit @ across)
        ys.extend(hit @ along)
    if not xs:
        return None
    return (min(xs), max(xs), min(ys), max(ys))


def axis(wall, frame):
    for rep in wall.Representation.Representations if wall.Representation else []:
        if rep.RepresentationIdentifier != "Axis":
            continue
        item = rep.Items[0]
        if item.is_a("IfcIndexedPolyCurve"):
            pts = item.Points.CoordList
        elif item.is_a("IfcPolyline"):
            pts = [p.Coordinates for p in item.Points]
        else:
            return None
        if len(pts) != 2:
            return None
        mat = ifcopenshell.util.placement.get_local_placement(wall.ObjectPlacement)
        world = np.array([(mat @ np.array([p[0], p[1], 0.0, 1.0]))[:3] for p in pts])
        a, b = frame(world)[:, :2]
        length = np.linalg.norm(b - a)
        if length < 1e-6:
            return None
        along = (b - a) / length
        return a, along, np.array([-along[1], along[0]])
    return None


def type_tags(f):
    out = {}
    for rel in f.by_type("IfcRelDefinesByType"):
        for obj in rel.RelatedObjects:
            if obj.is_a("IfcWall") and rel.RelatingType.Tag:
                out[obj.Tag] = rel.RelatingType.Tag
    return out


def body_sources(f):
    out = {}
    for wall in f.by_type("IfcWall"):
        for rel in wall.IsDefinedBy or []:
            pset = rel.RelatingPropertyDefinition
            if pset.is_a("IfcPropertySet"):
                for prop in pset.HasProperties:
                    if prop.Name == "BodySource":
                        out[wall.Tag] = prop.NominalValue.wrappedValue
    return out


def main(argv):
    args = argv[1:]
    listed = 10
    if "--list" in args:
        i = args.index("--list")
        listed = int(args[i + 1])
        del args[i : i + 2]
    if len(args) != 3:
        raise SystemExit(__doc__)
    probe_path, ours_path, revit_path = args
    types, words = {}, {}
    for line in open(probe_path):
        row = json.loads(line)
        if "type" in row:
            types[str(row["type"])] = row["angles"]
        else:
            words[str(row["wall"])] = row
    revit = ifcopenshell.open(revit_path)
    ours = ifcopenshell.open(ours_path)
    frame = site_frame(revit)
    walls = [w for w in revit.by_type("IfcWall") if w.Tag and axis(w, frame)]
    revit_meshes = meshes(revit, walls, frame)
    wall_types = type_tags(revit)

    tapered = []
    for wall in walls:
        mesh = revit_meshes.get(wall.Tag)
        if mesh is None:
            continue
        origin, along, across = axis(wall, frame)
        lo, hi = mesh[0][:, 2].min(), mesh[0][:, 2].max()
        base = cut(mesh, lo + 0.01 * (hi - lo), origin, along, across)
        top = cut(mesh, hi - 0.01 * (hi - lo), origin, along, across)
        if base is None or top is None:
            continue
        if (base[1] - base[0]) - (top[1] - top[0]) > TAPER_FEET:
            rise = 0.98 * (hi - lo)
            leans = (
                math.degrees(math.atan((top[0] - base[0]) / rise)),
                math.degrees(math.atan((base[1] - top[1]) / rise)),
            )
            tapered.append((wall, leans))

    print(f"{len(walls)} straight walls in Revit's export; {len(tapered)} wider at the base")
    word2 = {tag for tag, row in words.items() if row["word"] == 2}
    tapered_tags = {w.Tag for w, _ in tapered}
    print(f"  carrying word 2: {len(tapered_tags & word2)}; walls with word 2 not tapered: {len(word2 - tapered_tags)}")
    agree = 0
    for wall, leans in tapered:
        angles = types.get(wall_types.get(wall.Tag, ""))
        if angles is None:
            continue
        # One face leans by the type's middle angle and the other stands
        # vertical (within 0.01 degrees).
        want = math.degrees(angles[1])
        if sorted(abs(x) for x in leans)[0] < 0.01 and abs(max(leans, key=abs) - want) < 0.01:
            agree += 1
    print(f"  one face leaning by its type's angle, the other vertical: {agree}")

    our_meshes = meshes(ours, [w for w in ours.by_type("IfcWall") if w.Tag in tapered_tags], lambda v: v / FEET)
    sources = body_sources(ours)
    rows = []
    for wall, _ in tapered:
        mesh = revit_meshes[wall.Tag]
        other = our_meshes.get(wall.Tag)
        if other is None:
            rows.append((wall.Tag, None, None, sources.get(wall.Tag)))
            continue
        origin, along, across = axis(wall, frame)
        lo, hi = mesh[0][:, 2].min(), mesh[0][:, 2].max()
        worst_across = worst_along = 0.0
        for h in HEIGHTS:
            z = lo + h * (hi - lo)
            a = cut(mesh, z, origin, along, across)
            b = cut(other, z, origin, along, across)
            if a is None or b is None:
                worst_across = worst_along = math.inf
                break
            worst_across = max(worst_across, abs(a[0] - b[0]), abs(a[1] - b[1]))
            worst_along = max(worst_along, abs(a[2] - b[2]), abs(a[3] - b[3]))
        rows.append((wall.Tag, worst_across, worst_along, sources.get(wall.Tag)))

    compared = [r for r in rows if r[1] is not None]
    for title, subset in (
        ("the tapered walls", compared),
        ("the tapered walls with word 2", [r for r in compared if r[0] in word2]),
    ):
        by_source = {}
        for r in subset:
            by_source[r[3]] = by_source.get(r[3], 0) + 1
        print(f"rvt-rs bodies of {title}: {len(subset)}; BodySource {by_source}")
        for label, index in (("across", 1), ("along", 2)):
            counts = " ".join(
                f"{limit} ft: {sum(1 for r in subset if r[index] <= limit)}"
                for limit in (0.001, 0.01, 0.1, 1.0)
            )
            largest = max((r[index] for r in subset), default=0.0)
            print(f"  {label}, at all five heights within {counts}; max {largest:.3f} ft")
    for tag, a, b, source in sorted(compared, key=lambda r: -max(r[1], r[2]))[:listed]:
        print(f"  {tag}: across {a:.3f} ft, along {b:.3f} ft ({source})")


if __name__ == "__main__":
    main(sys.argv)

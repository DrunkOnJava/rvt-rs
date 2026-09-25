#!/usr/bin/env python3
"""Score the wall bodies in an `rvt-gltf` GLB against Revit's own IFC4 export.

Research tool for `reports/element-framing/RE-54-wall-bodies.md`. It is an
end-to-end check: the GLB comes from the built `rvt-gltf` binary, and the
reference is the body Revit's exporter wrote for the same wall, meshed by
IfcOpenShell.

For each wall present in both files, both bodies are measured in plan along
and across Revit's own wall direction (the X axis of the wall's placement,
turned into the model's frame by the site placement). It reports how far the
two faces (across) and the two ends (along) of rvt-rs's body lie from
Revit's, per wall, as the largest of the four differences. A wall drawn in
layers is several nodes; their union is the body.

A wall whose Revit axis is an arc (RE-75) is scored as curved instead: its
faces by the least and greatest distance of its body from the arc's centre,
and its ends by the body's first and last angle round it, as feet along the
arc at its middle radius.

A GLB node's name ends in the element's ElementId, which is the IFC `Tag`.

Usage:

    python3 tools/re/wall_bodies_vs_ifc.py <model.glb> <revit-export.ifc> [--list N] [--json OUT]

`--json` writes each compared wall's differences, for comparing two exports.

Needs IfcOpenShell (tested with 0.8.5).
"""

import collections
import importlib.util
import math
import os
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element
import ifcopenshell.util.placement

HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location(
    "wall_layers_vs_ifc", os.path.join(HERE, "wall_layers_vs_ifc.py")
)
layers = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(layers)


def wall_points(doc, blob):
    """ElementId -> [(x, y)] plan points of the wall's nodes, internal feet."""
    groups = collections.defaultdict(list)
    for node in doc["nodes"]:
        extras = node.get("extras") or {}
        if extras.get("ifcType") != "IFCWALL" or "mesh" not in node:
            continue
        groups[extras["entityIndex"]].append(node)
    out = {}
    for group in groups.values():
        name = group[0].get("name") or ""
        digits = name[len(name.rstrip("0123456789")) :]
        if not digits:
            continue
        points = []
        for node in group:
            prim = doc["meshes"][node["mesh"]]["primitives"][0]
            m4 = node["matrix"]
            for x, y, z in layers.accessor(doc, blob, prim["attributes"]["POSITION"]):
                points.append(
                    (
                        m4[0] * x + m4[4] * y + m4[8] * z + m4[12],
                        m4[1] * x + m4[5] * y + m4[9] * z + m4[13],
                    )
                )
        out[int(digits)] = points
    return out


def extents(points, d):
    n = (-d[1], d[0])
    along = [p[0] * d[0] + p[1] * d[1] for p in points]
    across = [p[0] * n[0] + p[1] * n[1] for p in points]
    return min(along), max(along), min(across), max(across)


def arc_centre(wall, to_internal):
    """The centre of the first arc in `wall`'s Axis representation, in model
    feet, or `None` for a wall whose axis is straight."""
    for rep in (wall.Representation.Representations if wall.Representation else []):
        if rep.RepresentationIdentifier != "Axis":
            continue
        for item in rep.Items:
            if not (item.is_a("IfcIndexedPolyCurve") and item.Segments):
                continue
            arcs = [s for s in item.Segments if s.is_a("IfcArcIndex")]
            if not arcs:
                continue
            placement = ifcopenshell.util.placement.get_local_placement(wall.ObjectPlacement)
            points = []
            for index in arcs[0].wrappedValue:
                x, y = item.Points.CoordList[index - 1][:2]
                world = placement @ [x, y, 0.0, 1.0]
                points.append(to_internal(world[0], world[1]))
            (ax, ay), (bx, by), (cx, cy) = points
            d = 2 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by))
            if abs(d) < 1e-12:
                return None
            ux = ((ax * ax + ay * ay) * (by - cy) + (bx * bx + by * by) * (cy - ay) + (cx * cx + cy * cy) * (ay - by)) / d
            uy = ((ax * ax + ay * ay) * (cx - bx) + (bx * bx + by * by) * (ax - cx) + (cx * cx + cy * cy) * (bx - ax)) / d
            return (ux, uy)
    return None


def polar(points, centre, reference):
    """The least and greatest distance from `centre`, and the least and
    greatest angle round it measured from `reference`, of `points`."""
    radii, angles = [], []
    for x, y in points:
        radii.append(math.hypot(x - centre[0], y - centre[1]))
        a = math.atan2(y - centre[1], x - centre[0]) - reference
        angles.append((a + math.pi) % (2 * math.pi) - math.pi)
    return min(radii), max(radii), min(angles), max(angles)


def main():
    args = sys.argv[1:]
    if len(args) < 2:
        raise SystemExit(__doc__)
    listing = int(args[args.index("--list") + 1]) if "--list" in args else 0
    json_out = args[args.index("--json") + 1] if "--json" in args else None
    doc, blob = layers.read_glb(args[0])
    ours = wall_points(doc, blob)
    f = ifcopenshell.open(args[1])
    to_internal, to_internal_direction = layers.site_to_internal(f)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    stats = collections.Counter()
    across_errors, along_errors, worst = [], [], []
    for wall in f.by_type("IfcWall"):
        try:
            tag = int(wall.Tag)
        except (TypeError, ValueError):
            continue
        if tag not in ours:
            continue
        try:
            shape = ifcopenshell.geom.create_shape(settings, wall)
        except Exception:
            stats["IfcOpenShell cannot mesh the IFC wall"] += 1
            continue
        verts = shape.geometry.verts
        theirs = [to_internal(verts[i], verts[i + 1]) for i in range(0, len(verts), 3)]
        if not theirs:
            stats["IFC wall has no body"] += 1
            continue
        placement = ifcopenshell.util.placement.get_local_placement(wall.ObjectPlacement)
        centre = arc_centre(wall, to_internal)
        if centre is not None:
            mx = sum(p[0] for p in theirs) / len(theirs)
            my = sum(p[1] for p in theirs) / len(theirs)
            reference = math.atan2(my - centre[1], mx - centre[0])
            a = polar(ours[tag], centre, reference)
            b = polar(theirs, centre, reference)
            middle = (b[0] + b[1]) / 2
            across = max(abs(a[0] - b[0]), abs(a[1] - b[1]))
            along = max(abs(a[2] - b[2]), abs(a[3] - b[3])) * middle
            kind = "curved"
        else:
            d = to_internal_direction(placement[0][0], placement[1][0])
            a = extents(ours[tag], d)
            b = extents(theirs, d)
            across = max(abs(a[2] - b[2]), abs(a[3] - b[3]))
            along = max(abs(a[0] - b[0]), abs(a[1] - b[1]))
            angled = min(abs(d[0]), abs(d[1])) > 1e-6
            kind = "angled" if angled else "axis-parallel"
        stats[f"compared, {kind}"] += 1
        across_errors.append((across, kind))
        along_errors.append((along, kind))
        wall_type = ifcopenshell.util.element.get_type(wall)
        worst.append(
            (round(max(across, along), 4), round(across, 4), round(along, 4), tag,
             kind, wall_type.Name if wall_type else None)
        )
    print(f"{len(ours)} walls in the GLB")
    for what, count in sorted(stats.items()):
        print(f"  {count:>5}  {what}")
    for label, errors in (("faces (across)", across_errors), ("ends (along)", along_errors)):
        for kind in ("axis-parallel", "angled", "curved"):
            es = sorted(e for e, k in errors if k == kind)
            if not es:
                continue
            within = " ".join(
                f"{t} ft: {sum(1 for e in es if e <= t)}" for t in (0.001, 0.01, 0.1, 1.0)
            )
            print(f"{label}, {kind}: {len(es)} walls; within {within}; max {es[-1]:.3f} ft")
    if json_out:
        import json

        with open(json_out, "w") as out:
            json.dump(
                {
                    str(row[3]): {"across": row[1], "along": row[2], "kind": row[4], "type": row[5]}
                    for row in worst
                },
                out,
            )
    if listing:
        print("largest differences (max, across, along, tag, kind, type):")
        for row in sorted(worst)[-listing:]:
            print(f"  {row}")


if __name__ == "__main__":
    main()

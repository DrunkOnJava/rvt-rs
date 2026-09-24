#!/usr/bin/env python3
"""Where each layer of a wall ends at a layered butt join, against Revit.

Research tool for `reports/element-framing/RE-71-layered-wall-joins.md`. Its
input is the JSON-lines output of `examples/probe_re71_layered_joins.rs`:
each wall's centreline, thickness, exterior side, layers (exterior first,
each width and function) and its RE-70 join at each end.

For every end with an RE-70 join where either wall has several layers, Revit's
IFC4 body is sliced in plan along the middle of each of the wall's layers,
and the layer's reach past the joint point is matched to the partner's layer
boundaries. The RE-71 rule counts both walls' layers from the outside of the
corner: layer `i` of the wall that runs through reaches the outer edge of the
partner's layer `i`, and layer `i` of the wall that stops, its inner edge. It
is scored on every end, split by whether the walls show the same layer
sequence from the corner and share their base and top (the ends the exporter
draws this way).

With `--glb MODEL.glb` (from `rvt-gltf`), each layer band the GLB draws is
also compared with Revit's body, at both ends, for every wall drawn in
layers.

Usage:

    python3 tools/re/wall_layer_joins_vs_ifc.py walls.jsonl revit-export.ifc [--glb model.glb]

Needs IfcOpenShell (tested with 0.8.5) and shapely.
"""

import collections
import importlib.util
import json
import math
import os
import sys

import ifcopenshell
import ifcopenshell.geom
from shapely.geometry import LineString, MultiPoint, Polygon
from shapely.ops import unary_union

HERE = os.path.dirname(os.path.abspath(__file__))
TOLERANCE = 2e-3
FUNCTIONS = {1: "structure", 2: "substrate", 3: "thermal", 4: "finish 1", 5: "finish 2"}


def load(name):
    spec = importlib.util.spec_from_file_location(name, os.path.join(HERE, name + ".py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def unit(v):
    length = math.hypot(*v)
    return (v[0] / length, v[1] / length)


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1]


def footprints(model, rows, to_internal):
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    out = {}
    for wall in model.by_type("IfcWall"):
        try:
            tag = int(wall.Tag)
        except (TypeError, ValueError):
            continue
        if tag not in rows:
            continue
        try:
            shape = ifcopenshell.geom.create_shape(settings, wall)
        except Exception:
            continue
        verts, faces = shape.geometry.verts, shape.geometry.faces
        points = [to_internal(verts[i], verts[i + 1]) for i in range(0, len(verts), 3)]
        triangles = [
            Polygon([points[faces[i]], points[faces[i + 1]], points[faces[i + 2]]])
            for i in range(0, len(faces), 3)
        ]
        out[tag] = unary_union([t for t in triangles if t.area > 1e-9]).buffer(0)
    return out


def span(geometry, origin, along):
    """The extent along `along` from `origin` of a line cut's pieces."""
    parts = (
        list(geometry.geoms)
        if geometry.geom_type.startswith("Multi") or geometry.geom_type == "GeometryCollection"
        else [geometry]
    )
    ts = [dot((x - origin[0], y - origin[1]), along) for g in parts if hasattr(g, "coords") for x, y in g.coords]
    return (min(ts), max(ts)) if ts else None


def boundaries(row):
    """Offsets of a wall's layer boundaries from its centreline towards its
    exterior, exterior face first."""
    at = row["thickness"] / 2
    out = [at]
    for width, _ in row["layers"]:
        at -= width
        out.append(at)
    return out


def rule_check(rows, bodies):
    counts = collections.Counter()
    by_layer = collections.Counter()
    for tag, row in rows.items():
        if tag not in bodies:
            continue
        start, end = row["start"], row["end"]
        d = unit((end[0] - start[0], end[1] - start[1]))
        n = unit(row["exterior"])
        for slot, joint in ((0, start), (1, end)):
            join = row["joins"][slot]
            other = join and rows.get(join[0])
            if not other or (len(row["layers"]) < 2 and len(other["layers"]) < 2):
                continue
            through = join[1]
            into = d if slot == 0 else (-d[0], -d[1])
            outward = 1.0 if dot(unit(other["exterior"]), (-into[0], -into[1])) >= 0 else -1.0
            partner_bounds = [b * outward for b in boundaries(other)]
            far = other["end"] if math.hypot(other["start"][0] - joint[0], other["start"][1] - joint[1]) < 1e-6 else other["start"]
            partner_on_exterior = dot((far[0] - joint[0], far[1] - joint[1]), n) > 0
            ours = [f for _, f in row["layers"]]
            if partner_on_exterior:
                ours = ours[::-1]
            theirs = [f for _, f in other["layers"]]
            if outward < 0:
                theirs = theirs[::-1]
            box, other_box = row.get("box"), other.get("box")
            same_height = bool(box and other_box) and abs(box[2] - other_box[2]) <= 1e-3 and abs(box[5] - other_box[5]) <= 1e-3
            drawn = ours == theirs and same_height and len(ours) >= 2
            ordered = sorted(partner_bounds, reverse=True)
            mids = [(a + b) / 2 for a, b in zip(boundaries(row), boundaries(row)[1:])]
            if partner_on_exterior:
                mids = mids[::-1]
            right = True
            for rank, mid in enumerate(mids):
                line = LineString([
                    (joint[0] + n[0] * mid - into[0] * 50, joint[1] + n[1] * mid - into[1] * 50),
                    (joint[0] + n[0] * mid + into[0] * 50, joint[1] + n[1] * mid + into[1] * 50),
                ])
                cut = span(bodies[tag].intersection(line), joint, into)
                index = rank if through else rank + 1
                ok = cut is not None and index < len(ordered) and abs(-cut[0] - ordered[index]) < TOLERANCE
                right &= ok
                by_layer[("drawn" if drawn else "not drawn", "right" if ok else "wrong")] += 1
            counts[("drawn" if drawn else "not drawn", "through" if through else "stops", "every layer right" if right else "a layer wrong")] += 1
    return counts, by_layer


def glb_check(rows, bodies, glb_path, helpers):
    doc, blob = helpers.read_glb(glb_path)
    nodes = collections.defaultdict(list)
    for node in doc["nodes"]:
        extras = node.get("extras") or {}
        if extras.get("ifcType") != "IFCWALL" or "mesh" not in node:
            continue
        name = node.get("name") or ""
        digits = name[len(name.rstrip("0123456789")):]
        if not digits:
            continue
        primitive = doc["meshes"][node["mesh"]]["primitives"][0]
        m = node["matrix"]
        nodes[int(digits)].append([
            (m[0] * x + m[4] * y + m[8] * z + m[12], m[1] * x + m[5] * y + m[9] * z + m[13])
            for x, y, z in helpers.accessor(doc, blob, primitive["attributes"]["POSITION"])
        ])
    counts = collections.Counter()
    for tag, row in rows.items():
        bands = nodes.get(tag, [])
        if tag not in bodies or len(bands) < 2 or len(bands) != len(row["layers"]):
            continue
        start, end = row["start"], row["end"]
        d = unit((end[0] - start[0], end[1] - start[1]))
        n = unit(row["exterior"])
        edges = boundaries(row)
        for band, upper, lower in zip(bands, edges, edges[1:]):
            mid = (upper + lower) / 2
            line = LineString([
                (start[0] + n[0] * mid - d[0] * 1e3, start[1] + n[1] * mid - d[1] * 1e3),
                (start[0] + n[0] * mid + d[0] * 1e3, start[1] + n[1] * mid + d[1] * 1e3),
            ])
            ours = span(MultiPoint(band).convex_hull.intersection(line), start, d)
            theirs = span(bodies[tag].intersection(line), start, d)
            if ours is None or theirs is None:
                counts["not measured"] += 1
                continue
            counts["right" if abs(ours[0] - theirs[0]) <= 1e-3 else "wrong"] += 1
            counts["right" if abs(ours[1] - theirs[1]) <= 1e-3 else "wrong"] += 1
    return counts


def main():
    args = sys.argv[1:]
    if len(args) < 2:
        raise SystemExit(__doc__)
    glb = args[args.index("--glb") + 1] if "--glb" in args else None
    rows = {row["id"]: row for row in map(json.loads, open(args[0]))}
    helpers = load("wall_layers_vs_ifc")
    model = ifcopenshell.open(args[1])
    to_internal, _ = helpers.site_to_internal(model)
    bodies = footprints(model, rows, to_internal)
    counts, by_layer = rule_check(rows, bodies)
    print("layered join ends against the rule, in Revit's body:")
    for key, count in sorted(counts.items()):
        print(f"  {count:>5}  {', '.join(key)}")
    for key, count in sorted(by_layer.items()):
        print(f"  {count:>5}  layers, {', '.join(key)}")
    if glb:
        print("GLB layer ends against Revit's body:")
        for key, count in sorted(glb_check(rows, bodies, glb, helpers).items()):
            print(f"  {count:>5}  {key}")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Score the wall layers in an `rvt-gltf` GLB against Revit's own IFC4 export.

Research tool for `reports/element-framing/RE-53-wall-layers.md`. It is an
end-to-end check: the GLB comes from the built `rvt-gltf` binary, and the
reference is the per-material solids Revit's exporter wrote for the same
file, meshed by IfcOpenShell.

`rvt-gltf` draws a wall whose layers it decoded as one node per layer,
exterior first, each with its layer's material ("Layer rrggbb", the
material's shading colour) or the wall's category material. Revit's IFC4
export writes the same wall with an `IfcMaterialConstituentSet`, one
constituent per layer in the same order, and its body's faces carry the
surface style of the layer they belong to. For every wall present in both,
the script measures each layer's span across the wall in both files and
compares the layer centres. A layer is compared only where its material
name occurs once in the wall, since two layers of one material cannot be
told apart by style. It also compares each compared layer's colour with the
`SurfaceColour` of Revit's style for it.

Coordinates: the GLB node matrices place bodies in Revit internal feet
(+Z up); IfcOpenShell world coordinates are the IFC's shared coordinates in
metres. The script takes the internal-to-shared transform from the
`IfcSite` placement Revit writes. A GLB node's name ends in the element's
ElementId, which is the IFC `Tag`.

Usage:

    python3 tools/re/wall_layers_vs_ifc.py <model.glb> <revit-export.ifc>

Needs IfcOpenShell (tested with 0.8.5).
"""

import collections
import json
import math
import re
import struct
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.placement

FEET = 0.3048
COMPONENT = {5121: "B", 5123: "H", 5125: "I", 5126: "f"}
WIDTH = {"SCALAR": 1, "VEC3": 3}


def read_glb(path):
    data = open(path, "rb").read()
    magic, _, _ = struct.unpack_from("<4sII", data, 0)
    if magic != b"glTF":
        raise SystemExit(f"{path}: not a GLB")
    offset, doc, blob = 12, None, b""
    while offset < len(data):
        length, kind = struct.unpack_from("<II", data, offset)
        chunk = data[offset + 8 : offset + 8 + length]
        if kind == 0x4E4F534A:
            doc = json.loads(chunk)
        elif kind == 0x004E4942:
            blob = chunk
        offset += 8 + length
    return doc, blob


def accessor(doc, blob, index):
    acc = doc["accessors"][index]
    view = doc["bufferViews"][acc["bufferView"]]
    fmt = COMPONENT[acc["componentType"]]
    n = WIDTH[acc["type"]]
    start = view.get("byteOffset", 0) + acc.get("byteOffset", 0)
    values = struct.unpack_from(f"<{acc['count'] * n}{fmt}", blob, start)
    return [values[i : i + n] for i in range(0, len(values), n)]


def layered_walls(doc, blob):
    """ElementId -> [(material name, [(x, y) internal feet])], exterior first.

    A wall drawn in layers is one node per layer; a single-layer wall is one
    node whose material is its layer's colour."""
    nodes = collections.defaultdict(list)
    for node in doc["nodes"]:
        extras = node.get("extras") or {}
        if extras.get("ifcType") != "IFCWALL" or "mesh" not in node:
            continue
        nodes[extras["entityIndex"]].append(node)
    out = {}
    for group in nodes.values():
        m = re.search(r"(\d+)$", group[0].get("name") or "")
        if not m:
            continue
        layers = []
        for node in group:
            prim = doc["meshes"][node["mesh"]]["primitives"][0]
            material = doc["materials"][prim["material"]].get("name", "")
            m4 = node["matrix"]
            points = [
                (
                    m4[0] * x + m4[4] * y + m4[8] * z + m4[12],
                    m4[1] * x + m4[5] * y + m4[9] * z + m4[13],
                )
                for x, y, z in accessor(doc, blob, prim["attributes"]["POSITION"])
            ]
            layers.append((material, points))
        if len(layers) > 1 or layers[0][0].startswith("Layer "):
            out[int(m.group(1))] = layers
    return out


def wall_materials(wall):
    """The material names Revit's export gives a wall, in layer order."""
    sources = [wall] + [
        rel.RelatingType for rel in getattr(wall, "IsTypedBy", None) or []
    ]
    for source in sources:
        for rel in getattr(source, "HasAssociations", None) or []:
            if not rel.is_a("IfcRelAssociatesMaterial"):
                continue
            m = rel.RelatingMaterial
            if m.is_a("IfcMaterialConstituentSet"):
                return [k.Material.Name for k in m.MaterialConstituents]
            if m.is_a("IfcMaterialLayerSetUsage"):
                m = m.ForLayerSet
            if m.is_a("IfcMaterialLayerSet"):
                return [layer.Material.Name for layer in m.MaterialLayers]
            if m.is_a("IfcMaterial"):
                return [m.Name]
    return None


def site_to_internal(f):
    """Shared metres (IfcOpenShell world) -> Revit internal feet, in plan,
    for points and for directions."""
    site = f.by_type("IfcSite")[0]
    mat = ifcopenshell.util.placement.get_local_placement(site.ObjectPlacement)
    ox, oy = mat[0][3], mat[1][3]
    c, s = mat[0][0], mat[1][0]

    def point(x, y):
        dx, dy = x - ox, y - oy
        return ((c * dx + s * dy) / FEET, (-s * dx + c * dy) / FEET)

    def direction(x, y):
        u, v = c * x + s * y, -s * x + c * y
        norm = math.hypot(u, v)
        return (u / norm, v / norm)

    return point, direction


def span(points, n):
    values = [p[0] * n[0] + p[1] * n[1] for p in points]
    return min(values), max(values)


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    doc, blob = read_glb(sys.argv[1])
    ours = layered_walls(doc, blob)
    f = ifcopenshell.open(sys.argv[2])
    to_internal, to_internal_direction = site_to_internal(f)
    style = {}
    for st in f.by_type("IfcSurfaceStyle"):
        for item in st.Styles or []:
            colour = getattr(item, "SurfaceColour", None)
            rgb = None
            if colour is not None:
                rgb = "%02x%02x%02x" % tuple(
                    round(255 * v) for v in (colour.Red, colour.Green, colour.Blue)
                )
            style[f"{item.is_a()}-{item.id()}"] = (st.Name, rgb)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)

    stats = collections.Counter()
    errors, worst = [], []
    colours = collections.Counter()
    differs = []
    for wall in f.by_type("IfcWall"):
        try:
            tag = int(wall.Tag)
        except (TypeError, ValueError):
            continue
        if tag not in ours:
            continue
        stats["drawn in layers in the GLB and present in the IFC"] += 1
        layers = ours[tag]
        constituents = wall_materials(wall)
        if not constituents:
            stats["IFC wall has no material"] += 1
            continue
        if len(constituents) != len(layers):
            stats["layer count differs"] += 1
            continue
        # Across the wall: Revit's wall placement's local +Y, turned to point
        # from our last layer to our first (exterior first).
        placement = ifcopenshell.util.placement.get_local_placement(wall.ObjectPlacement)
        axis = to_internal_direction(placement[0][1], placement[1][1])
        if len(layers) > 1:
            first, last = span(layers[0][1], axis), span(layers[-1][1], axis)
            if sum(first) < sum(last):
                axis = (-axis[0], -axis[1])
        n = axis
        try:
            shape = ifcopenshell.geom.create_shape(settings, wall)
        except Exception:
            stats["IfcOpenShell cannot mesh the IFC wall"] += 1
            continue
        g = shape.geometry
        verts, faces, ids = g.verts, g.faces, g.material_ids
        names = [style.get(m.name, (m.name, None)) for m in g.materials]
        theirs = collections.defaultdict(list)
        for t in range(len(faces) // 3):
            mi = ids[t] if t < len(ids) else -1
            if not 0 <= mi < len(names):
                continue
            for k in range(3):
                v = faces[3 * t + k]
                theirs[names[mi][0]].append(to_internal(verts[3 * v], verts[3 * v + 1]))
        rgb_of = {name: rgb for name, rgb in names}
        counts = collections.Counter(constituents)
        wall_error, compared = 0.0, 0
        for i, name in enumerate(constituents):
            if counts[name] > 1 or name not in theirs:
                continue
            lo, hi = span(theirs[name], n)
            olo, ohi = span(layers[i][1], n)
            wall_error = max(wall_error, abs((lo + hi) / 2 - (olo + ohi) / 2))
            compared += 1
            material = layers[i][0]
            if material.startswith("Layer ") and rgb_of.get(name):
                same = material[6:] == rgb_of[name]
                colours["same" if same else "different"] += 1
                if not same and len(differs) < 6:
                    differs.append((tag, name, material[6:], rgb_of[name]))
            elif material.startswith("Layer "):
                colours["IFC style has no colour"] += 1
            else:
                colours["drawn in the category colour"] += 1
        if not compared:
            stats["no layer with a unique material"] += 1
            continue
        stats["compared"] += 1
        stats[f"compared, {'one layer' if len(layers) == 1 else 'two or more layers'}"] += 1
        errors.append(wall_error)
        worst.append((round(wall_error, 4), tag))

    print(f"{len(ours)} walls drawn in layers in the GLB")
    print(f"  {sum(1 for v in ours.values() if len(v) == 1):>5}  of them one layer")
    for what, count in stats.items():
        print(f"  {count:>5}  {what}")
    errors.sort()
    if errors:
        for t in (0.001, 0.01, 0.05, 0.25):
            within = sum(1 for e in errors if e <= t)
            print(f"  layer centres within {t} ft: {within} of {len(errors)}")
        print(f"  median {errors[len(errors) // 2]:.2e} ft, max {errors[-1]:.4f} ft")
        print(f"  worst {sorted(worst)[-6:]}")
    print("compared layers' colours against Revit's surface styles:")
    for what, count in colours.items():
        print(f"  {count:>5}  {what}")
    for row in differs:
        print(f"  different: {row}")


if __name__ == "__main__":
    main()

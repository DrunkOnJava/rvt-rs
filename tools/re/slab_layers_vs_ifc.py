#!/usr/bin/env python3
"""Score floors', roofs' and ceilings' layers in an `rvt-gltf` GLB against
Revit's own IFC4 export.

Research tool for `reports/element-framing/RE-57-slab-layers.md`, the
stacked counterpart of `wall_layers_vs_ifc.py`. `rvt-gltf` draws a slab
whose layers it read as one node per layer, top first. Revit's IFC4 export
styles each face of the same element's body with its layer's material. For
every element present in both, the script measures each layer's height span
in both files, where the layer's material occurs once in the element, and
compares the layer's bottom and top. It also compares each compared layer's
colour with Revit's surface style. Where Revit styles a layered body with a
single material, only the colours are compared, in Revit's layer order.

Heights: the GLB node matrices place bodies in Revit internal feet; the IFC's
are shared coordinates in metres, and the `IfcSite` placement's elevation is
subtracted.

Usage:

    python3 tools/re/slab_layers_vs_ifc.py <model.glb> <revit-export.ifc>

Needs IfcOpenShell (tested with 0.8.5).
"""

import collections
import importlib.util
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
walls = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(walls)

FEET = 0.3048
STACKED = {"IFCSLAB", "IFCROOF", "IFCCOVERING"}


def stacked_layers(doc, blob):
    """ElementId -> (ifc type, [(material name, (low z, high z))]), top first."""
    groups = collections.defaultdict(list)
    for node in doc["nodes"]:
        extras = node.get("extras") or {}
        if extras.get("ifcType") not in STACKED or "mesh" not in node:
            continue
        groups[extras["entityIndex"]].append(node)
    out = {}
    for group in groups.values():
        name = group[0].get("name") or ""
        digits = name[len(name.rstrip("0123456789")) :]
        if not digits:
            continue
        layers = []
        for node in group:
            prim = doc["meshes"][node["mesh"]]["primitives"][0]
            material = doc["materials"][prim["material"]].get("name", "")
            m4 = node["matrix"]
            zs = [
                m4[2] * x + m4[6] * y + m4[10] * z + m4[14]
                for x, y, z in walls.accessor(doc, blob, prim["attributes"]["POSITION"])
            ]
            layers.append((material, (min(zs), max(zs))))
        if len(layers) > 1 or layers[0][0].startswith("Layer "):
            out[int(digits)] = (group[0]["extras"]["ifcType"], layers)
    return out


def element_materials(element):
    """Material names of the element's layers as Revit's export orders them."""
    material = ifcopenshell.util.element.get_material(element, should_skip_usage=True)
    if material is None:
        return None
    if material.is_a("IfcMaterialConstituentSet"):
        return [k.Material.Name for k in material.MaterialConstituents]
    if material.is_a("IfcMaterialLayerSet"):
        return [layer.Material.Name if layer.Material else None for layer in material.MaterialLayers]
    if material.is_a("IfcMaterial"):
        return [material.Name]
    return None


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    doc, blob = walls.read_glb(sys.argv[1])
    ours = stacked_layers(doc, blob)
    f = ifcopenshell.open(sys.argv[2])
    site = f.by_type("IfcSite")[0]
    site_z = ifcopenshell.util.placement.get_local_placement(site.ObjectPlacement)[2][3]
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
    colours = collections.Counter()
    errors = []
    by_tag = {}
    for element in f.by_type("IfcBuildingElement"):
        if element.Tag and element.Tag.isdigit():
            by_tag.setdefault(int(element.Tag), element)
    for tag, (kind, layers) in sorted(ours.items()):
        element = by_tag.get(tag)
        if element is None:
            stats["not in the IFC"] += 1
            continue
        stats[f"{kind} in both"] += 1
        materials = element_materials(element)
        try:
            shape = ifcopenshell.geom.create_shape(settings, element)
        except Exception:
            stats["IfcOpenShell cannot mesh the IFC element"] += 1
            continue
        g = shape.geometry
        verts, faces, ids = g.verts, g.faces, g.material_ids
        names = [style.get(m.name, (m.name, None)) for m in g.materials]
        spans = collections.defaultdict(lambda: [float("inf"), float("-inf")])
        for t in range(len(faces) // 3):
            mi = ids[t] if t < len(ids) else -1
            if not 0 <= mi < len(names):
                continue
            for k in range(3):
                z = (verts[3 * faces[3 * t + k] + 2] - site_z) / FEET
                span = spans[names[mi][0]]
                span[0], span[1] = min(span[0], z), max(span[1], z)
        rgb_of = {name: rgb for name, rgb in names}
        if not materials or len(materials) != len(layers):
            # One layer: compare against the whole body.
            if len(layers) == 1 and len(spans) == 1:
                (name, span), = spans.items()
                materials = [name]
            else:
                stats["layer count differs from Revit's"] += 1
                continue
        if len(layers) > 1 and len(spans) == 1:
            # Revit styled the whole body with one material: the layers'
            # heights cannot be told apart, only their colours in order.
            stats["Revit styles the whole body with one material"] += 1
            for (material, _), name in zip(layers, materials):
                if material.startswith("Layer ") and rgb_of.get(name):
                    colours["same" if material[6:] == rgb_of[name] else "different"] += 1
            continue
        counts = collections.Counter(materials)
        worst, compared = 0.0, 0
        for (material, (low, high)), name in zip(layers, materials):
            if counts[name] > 1 or name not in spans:
                continue
            lo, hi = spans[name]
            worst = max(worst, abs(lo - low), abs(hi - high))
            compared += 1
            if material.startswith("Layer ") and rgb_of.get(name):
                colours["same" if material[6:] == rgb_of[name] else "different"] += 1
        if not compared:
            stats["no layer with a unique material"] += 1
            continue
        stats["compared"] += 1
        errors.append((worst, tag, kind))
    print(f"{len(ours)} floors, roofs and ceilings drawn in layers in the GLB")
    for what, count in sorted(stats.items()):
        print(f"  {count:>5}  {what}")
    es = sorted(e for e, _, _ in errors)
    if es:
        for t in (0.001, 0.01, 0.1):
            print(f"  layer bottoms and tops within {t} ft: {sum(1 for e in es if e <= t)} of {len(es)}")
        print(f"  max {es[-1]:.4f} ft; worst {sorted(errors)[-5:]}")
    print("compared layers' colours against Revit's surface styles:")
    for what, count in colours.items():
        print(f"  {count:>5}  {what}")


if __name__ == "__main__":
    main()

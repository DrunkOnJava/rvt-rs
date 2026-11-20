"""Newly drawn saved-mesh bodies against the record box the base drew for the same object.

    python box_check.py <base model.ifc> <head model.ifc>

For every object whose BodySource moves from a record box (`partition_element_record_bbox` or
`partition_family_instance_turned_box`) to `saved_graphics_mesh`, both bodies are meshed in world
coordinates. The base body is a box: its three face directions are read from its triangles'
normals, and the head mesh's vertices are projected on them. Per object it reports, in the file's
length unit, the largest overhang (how far the mesh reaches past the box on any side) and the
largest gap (how far short of a box face the mesh stops). A mesh in its own record box has no
overhang; a mesh filling it has no gap either.
"""
import collections
import importlib.util
import os
import sys

import ifcopenshell
import ifcopenshell.geom
import numpy as np

spec = importlib.util.spec_from_file_location("ifc_body_diff", os.path.join(os.path.dirname(os.path.abspath(__file__)), "ifc_body_diff.py"))
body_diff = importlib.util.module_from_spec(spec)
spec.loader.exec_module(body_diff)
BOXES = ("partition_element_record_bbox", "partition_family_instance_turned_box")


def mesh(settings, element):
    shape = ifcopenshell.geom.create_shape(settings, element)
    verts = np.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
    faces = np.array(shape.geometry.faces, dtype=int).reshape(-1, 3)
    return verts, faces


def box_axes(verts, faces):
    p = verts[faces]
    normals = np.cross(p[:, 1] - p[:, 0], p[:, 2] - p[:, 0])
    lengths = np.linalg.norm(normals, axis=1)
    axes = []
    for n, length in zip(normals, lengths):
        if length < 1e-12:
            continue
        n = n / length
        if not any(abs(float(np.dot(n, a))) > 0.999 for a in axes):
            axes.append(n)
    return axes


def main():
    base_path, head_path = sys.argv[1], sys.argv[2]
    base_props, head_props = body_diff.read(base_path), body_diff.read(head_path)
    moved = [g for g, h in head_props.items() if "'saved_graphics_mesh'" in h[2] and g in base_props and any(f"'{b}'" in base_props[g][2] for b in BOXES)]
    base, head = ifcopenshell.open(base_path), ifcopenshell.open(head_path)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    rows = []
    for gid in moved:
        try:
            bv, bf = mesh(settings, base.by_guid(gid))
            hv, _ = mesh(settings, head.by_guid(gid))
        except Exception as err:  # a body IfcOpenShell cannot mesh is reported, not skipped silently
            rows.append((head_props[gid][0], base_props[gid][2].replace("IFCTEXT('", '').rstrip("')"), None, None, f"mesh error {err}"))
            continue
        axes = box_axes(bv, bf)
        if len(axes) != 3:
            rows.append((head_props[gid][0], base_props[gid][2].replace("IFCTEXT('", '').rstrip("')"), None, None, f"base body has {len(axes)} face directions"))
            continue
        over = gap = 0.0
        for a in axes:
            b, h = bv @ a, hv @ a
            over = max(over, b.min() - h.min(), h.max() - b.max())
            gap = max(gap, h.min() - b.min(), b.max() - h.max())
        rows.append((head_props[gid][0], base_props[gid][2].replace("IFCTEXT('", '').rstrip("')"), max(over, 0.0), max(gap, 0.0), head_props[gid][1]))
    print(f"moved from a record box to a saved mesh: {len(moved)}")
    by = collections.defaultdict(list)
    for kind, source, over, gap, tag in rows:
        by[(kind, source)].append((over, gap, tag))
    for (kind, source), items in sorted(by.items()):
        good = [x for x in items if x[0] is not None]
        bad = [x for x in items if x[0] is None]
        overs = sorted(x[0] for x in good)
        gaps = sorted(x[1] for x in good)
        line = f"{kind:24} {source:38} n={len(items):4}"
        if good:
            line += (f" overhang max={overs[-1]:.4f} median={overs[len(overs) // 2]:.4f} >0.01:{sum(o > 0.01 for o in overs):4}"
                     f" | gap max={gaps[-1]:.4f} median={gaps[len(gaps) // 2]:.4f} >0.01:{sum(g > 0.01 for g in gaps):4}")
        if bad:
            line += f" | not read {len(bad)}: {bad[0][2]}"
        print(line)
    worst = sorted((x for x in rows if x[2] is not None), key=lambda r: -r[2])[:5]
    for kind, source, over, gap, tag in worst:
        print(f"  largest overhang: {kind} Tag {tag} overhang {over:.4f} gap {gap:.4f}")


main()

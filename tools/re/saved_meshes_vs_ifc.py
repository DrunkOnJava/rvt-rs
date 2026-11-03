#!/usr/bin/env python3
"""Score rvt-rs bodies against Revit's own IFC export, per element (#255).

Research tool for `reports/element-framing/RE-78-saved-graphics-meshes.md`.
For every element of an rvt-rs IFC whose `BodySource` is one of the given
sources, it compares the world axis-aligned box of the body with the box of
the element with the same `Tag` in Revit's export, and buckets the largest
of the six corner differences (feet). `IfcOpeningElement` is skipped on
both sides: Revit tags an opening with the id of the element that cuts it.

Each file's bodies are taken in Revit's internal feet by undoing that
file's own IfcSite placement (`ifc_common.site_frame`), as the other body
scorers do, so a reference export in shared coordinates, turned to true
north (the BIM4LCA office), compares box for box. With `--rotate-deg` or
`--translate-ft X,Y,Z`, rvt-rs's bodies are instead moved into the
reference's world coordinates by that internal-to-shared transform, and the
reference's are taken as they are. Either way any residual is removed as
one translation for the whole file: `--offset-ft`, or by default the median
difference over the scored elements. The offset is in the frame the boxes
are compared in: internal feet by default, the reference's world feet with
a transform.

Only the rvt-rs elements with a wanted `BodySource`, and the reference's
elements with their Tags, are meshed.

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).

Usage:

    python3 tools/re/saved_meshes_vs_ifc.py <rvt-rs.ifc> <revit.ifc> \
        [--source saved_graphics_mesh] [--rotate-deg DEG] \
        [--translate-ft X,Y,Z] [--offset-ft X,Y,Z]
"""

import argparse
import collections
import math
import os
import statistics
import sys

import ifcopenshell
import ifcopenshell.geom
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ifc_common import mesh_parts, site_frame  # noqa: E402

FT = 0.3048


def body_sources(element):
    for rel in element.IsDefinedBy:
        if not rel.is_a("IfcRelDefinesByProperties"):
            continue
        for prop in getattr(rel.RelatingPropertyDefinition, "HasProperties", None) or []:
            if prop.Name == "BodySource" and prop.NominalValue is not None:
                return prop.NominalValue.wrappedValue
    return None


def world_boxes(path, tags=None, transform=None, wanted=None, frame=True):
    """Tag -> (class, box in feet, BodySource) for elements with a body: in
    the file's internal feet through its site placement (`frame`), else in
    its world feet, moved by `transform` when given. With `wanted`, only the
    elements with one of those BodySources are meshed."""
    model = ifcopenshell.open(path)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    elements = [
        e
        for e in model.by_type("IfcElement")
        if not e.is_a("IfcOpeningElement")
        and e.Tag
        and e.Representation
        and (tags is None or e.Tag in tags)
    ]
    sources = {e.Tag: body_sources(e) for e in elements}
    if wanted is not None:
        elements = [e for e in elements if sources.get(e.Tag) in wanted]
    out = {}
    if not elements:
        return out
    to_internal = site_frame(model) if frame else None
    # The order the geometry iterator returned them in, one thread, which the
    # tables' ties (equal counts) follow.
    elements.sort(key=lambda e: e.id())
    shapes = mesh_parts(model, settings, elements, path)
    for element in elements:
        if element.id() not in shapes:
            continue
        verts = shapes[element.id()][0]
        if len(verts):
            if to_internal is not None:
                points = to_internal(verts)
            else:
                points = verts / FT
                if transform:
                    points = np.array([transform(p) for p in points])
            box = [float(v) for v in points.min(axis=0)] + [float(v) for v in points.max(axis=0)]
            out[element.Tag] = (element.is_a(), box, sources.get(element.Tag))
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("ours")
    ap.add_argument("reference")
    ap.add_argument("--source", action="append", default=None)
    ap.add_argument("--rotate-deg", type=float, default=None)
    ap.add_argument("--translate-ft", default=None)
    ap.add_argument("--offset-ft", default=None)
    args = ap.parse_args()
    wanted = set(args.source or ["saved_graphics_mesh"])
    frames = args.rotate_deg is None and args.translate_ft is None
    tx, ty, tz = (float(v) for v in (args.translate_ft or "0,0,0").split(","))
    angle = math.radians(args.rotate_deg or 0.0)
    c, s = math.cos(angle), math.sin(angle)

    def to_shared(p):
        x, y, z = p
        return (x * c - y * s + tx, x * s + y * c + ty, z + tz)

    ours = world_boxes(args.ours, transform=None if frames else to_shared, wanted=wanted, frame=frames)
    ref = world_boxes(args.reference, tags=set(ours), frame=frames)
    print("frames:", "each file's site placement undone" if frames else "rvt-rs moved by the given transform")
    pairs = [(ours[t][1], ref[t][1]) for t in ours if t in ref]
    if args.offset_ft:
        offset = [float(v) for v in args.offset_ft.split(",")]
    else:
        offset = [statistics.median(a[k] - b[k] for a, b in pairs) for k in range(3)] if pairs else [0.0] * 3
    offset *= 2
    print("file-wide offset removed (ft):", [round(v, 4) for v in offset[:3]])

    rows = collections.defaultdict(lambda: [0] * 6)
    for tag, (cls, box, _) in ours.items():
        row = rows[cls]
        row[0] += 1
        if tag not in ref:
            row[5] += 1
            continue
        err = max(abs(box[k] - offset[k] - ref[tag][1][k]) for k in range(6))
        row[1 if err <= 0.01 else 2 if err <= 0.1 else 3 if err <= 1 else 4] += 1
    widths = (6, 10, 8, 8, 7, 8)
    print(f"{'class':28}" + "".join(f"{h:>{w}}" for h, w in zip(("n", "<=0.01ft", "<=0.1", "<=1", ">1", "no ref"), widths)))
    total = [0] * 6
    for cls, row in sorted(rows.items(), key=lambda kv: -kv[1][0]):
        print(f"{cls:28}" + "".join(f"{v:>{w}}" for v, w in zip(row, widths)))
        total = [a + b for a, b in zip(total, row)]
    print(f"{'total':28}" + "".join(f"{v:>{w}}" for v, w in zip(total, widths)))


if __name__ == "__main__":
    main()

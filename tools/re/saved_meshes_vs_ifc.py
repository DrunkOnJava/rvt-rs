#!/usr/bin/env python3
"""Score rvt-rs bodies against Revit's own IFC export, per element (#255).

Research tool for `reports/element-framing/RE-78-saved-graphics-meshes.md`.
For every element of an rvt-rs IFC whose `BodySource` is one of the given
sources, it compares the world axis-aligned box of the body with the box of
the element with the same `Tag` in Revit's export, and buckets the largest
of the six corner differences (feet). `IfcOpeningElement` is skipped on
both sides: Revit tags an opening with the id of the element that cuts it.

rvt-rs writes Revit's internal coordinates. When the reference export uses
shared coordinates, pass the internal-to-shared transform (`--rotate-deg`,
`--translate-ft X,Y,Z`). A published transform is rounded; `--offset-ft`
removes the residual as one translation for the whole file, and without it
the residual is taken as the median difference over the scored elements.

Needs IfcOpenShell 0.8.5 (`pip install ifcopenshell==0.8.5`).

Usage:

    python3 tools/re/saved_meshes_vs_ifc.py <rvt-rs.ifc> <revit.ifc> \
        [--source saved_graphics_mesh] [--rotate-deg 0] \
        [--translate-ft 0,0,0] [--offset-ft X,Y,Z]
"""

import argparse
import collections
import math
import multiprocessing
import statistics

import ifcopenshell
import ifcopenshell.geom

FT = 0.3048


def body_sources(element):
    for rel in element.IsDefinedBy:
        if not rel.is_a("IfcRelDefinesByProperties"):
            continue
        for prop in getattr(rel.RelatingPropertyDefinition, "HasProperties", None) or []:
            if prop.Name == "BodySource" and prop.NominalValue is not None:
                return prop.NominalValue.wrappedValue
    return None


def world_boxes(path, tags=None, transform=None):
    """Tag -> (class, box in feet, BodySource) for elements with a body."""
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
    out = {}
    it = ifcopenshell.geom.iterator(settings, model, multiprocessing.cpu_count(), include=elements)
    if not it.initialize():
        return out
    while True:
        shape = it.get()
        verts = shape.geometry.verts
        points = [(verts[i] / FT, verts[i + 1] / FT, verts[i + 2] / FT) for i in range(0, len(verts), 3)]
        if points:
            if transform:
                points = [transform(p) for p in points]
            box = [min(p[k] for p in points) for k in range(3)] + [max(p[k] for p in points) for k in range(3)]
            element = model.by_id(shape.id)
            out[element.Tag] = (element.is_a(), box, sources.get(element.Tag))
        if not it.next():
            break
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("ours")
    ap.add_argument("reference")
    ap.add_argument("--source", action="append", default=None)
    ap.add_argument("--rotate-deg", type=float, default=0.0)
    ap.add_argument("--translate-ft", default="0,0,0")
    ap.add_argument("--offset-ft", default=None)
    args = ap.parse_args()
    wanted = set(args.source or ["saved_graphics_mesh"])
    tx, ty, tz = (float(v) for v in args.translate_ft.split(","))
    c, s = math.cos(math.radians(args.rotate_deg)), math.sin(math.radians(args.rotate_deg))

    def to_shared(p):
        x, y, z = p
        return (x * c - y * s + tx, x * s + y * c + ty, z + tz)

    ours = {t: v for t, v in world_boxes(args.ours, transform=to_shared).items() if v[2] in wanted}
    ref = world_boxes(args.reference, tags=set(ours))
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

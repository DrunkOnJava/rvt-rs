#!/usr/bin/env python3
"""How much of its own bounding box Revit's body fills, for every element
rvt-rs draws as its record box, per IFC class (W6 census).

For each element of an rvt-rs IFC whose `BodySource` is
`partition_element_record_bbox`, the element with the same `Tag` in
Revit's export is meshed (world coordinates) and its volume divided by the
volume of its axis-aligned box. A fill of 100% means Revit's body is a box
along the model axes, so a record box with Revit's extents is already its
shape; less means the shape is not decoded. The ratio needs no coordinate
transform, so it also reads exports in shared coordinates.

Buckets: `box` (fill within 0.5% of 100%), `90%`, `50%`, `<50%`, `>100%`
(a fill over 100.5%: overlapping solids or an open mesh, so the volume is
not the body's), and `no ref` (no element with that Tag, or no body). Every class is listed, most
numerous first, with its median fill.

Usage:

    python3 tools/re/box_bodies_vs_ifc.py <rvt-rs.ifc> <revit.ifc>
"""

import collections
import multiprocessing
import os
import statistics
import sys

import ifcopenshell
import ifcopenshell.geom

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from saved_meshes_vs_ifc import body_sources  # noqa: E402

BOX_SOURCE = "partition_element_record_bbox"


def boxed_tags(path):
    model = ifcopenshell.open(path)
    return {
        e.Tag: e.is_a()
        for e in model.by_type("IfcElement")
        if e.Tag and not e.is_a("IfcOpeningElement") and body_sources(e) == BOX_SOURCE
    }


def fills(path, tags):
    """Tag -> volume / bounding-box volume of Revit's body."""
    model = ifcopenshell.open(path)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    elements = [
        e
        for e in model.by_type("IfcElement")
        if e.Tag in tags and not e.is_a("IfcOpeningElement") and e.Representation
    ]
    out = {}
    if not elements:
        return out
    it = ifcopenshell.geom.iterator(settings, model, multiprocessing.cpu_count(), include=elements)
    if not it.initialize():
        return out
    while True:
        shape = it.get()
        v = shape.geometry.verts
        f = shape.geometry.faces
        points = [(v[i], v[i + 1], v[i + 2]) for i in range(0, len(v), 3)]
        if points and f:
            volume = 0.0
            for i in range(0, len(f), 3):
                a, b, c = points[f[i]], points[f[i + 1]], points[f[i + 2]]
                volume += (
                    a[0] * (b[1] * c[2] - b[2] * c[1])
                    - a[1] * (b[0] * c[2] - b[2] * c[0])
                    + a[2] * (b[0] * c[1] - b[1] * c[0])
                ) / 6.0
            extent = [max(p[k] for p in points) - min(p[k] for p in points) for k in range(3)]
            box = extent[0] * extent[1] * extent[2]
            tag = model.by_id(shape.id).Tag
            if box > 0:
                out[tag] = out.get(tag, 0.0) + abs(volume) / box
        if not it.next():
            break
    return out


def main():
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    boxed = boxed_tags(sys.argv[1])
    fill = fills(sys.argv[2], set(boxed))
    rows = collections.defaultdict(lambda: collections.Counter())
    medians = collections.defaultdict(list)
    for tag, cls in boxed.items():
        f = fill.get(tag)
        row = rows[cls]
        row["n"] += 1
        if f is None:
            row["no ref"] += 1
            continue
        medians[cls].append(f)
        row[
            ">100%"
            if f > 1.005
            else "box"
            if f >= 0.995
            else "90%"
            if f >= 0.9
            else "50%"
            if f >= 0.5
            else "<50%"
        ] += 1
    heads = ("n", "box", "90%", "50%", "<50%", ">100%", "no ref")
    print(f"record-box bodies: {len(boxed)}; fill = Revit's body volume / its bounding box")
    print(f"{'class':28}" + "".join(f"{h:>8}" for h in heads) + f"{'median':>9}")
    total = collections.Counter()
    for cls, row in sorted(rows.items(), key=lambda kv: -kv[1]["n"]):
        med = statistics.median(medians[cls]) if medians[cls] else float("nan")
        print(f"{cls:28}" + "".join(f"{row[h]:>8}" for h in heads) + f"{med:>9.3f}")
        total.update(row)
    print(f"{'total':28}" + "".join(f"{total[h]:>8}" for h in heads))
    return 0


if __name__ == "__main__":
    sys.exit(main())

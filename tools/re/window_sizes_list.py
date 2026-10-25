#!/usr/bin/env python3
"""Where a window's OverallWidth and OverallHeight in Revit's export come
from, where its symbol holds no cut loop (#810).

Takes the output of `examples/probe_re187_door_window_sizes.rs` (each
window's type Width and Height, -1001301 and -1001300, and its record box)
and Revit's export. For each window type it prints, once per distinct
combination: Revit's OverallWidth and OverallHeight; the type's Width and
Height; the record box; and the extents of the window's own body and of the
opening it fills, each in its own placement's frame, in feet.

Needs IfcOpenShell and NumPy.

Usage:

    python3 tools/re/window_sizes_list.py <probe output> <revit.ifc>
"""

import collections
import json
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.unit
import numpy

FEET = 0.3048


def local_extents(settings, element):
    """The element's body's extents in its own placement's frame, feet (the
    geometry kernel works in metres, whatever the file's unit)."""
    try:
        shape = ifcopenshell.geom.create_shape(settings, element)
    except Exception:
        return None
    v = numpy.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
    if not len(v):
        return None
    return tuple(round(float(x) / FEET, 4) for x in (v.max(axis=0) - v.min(axis=0)))


def main():
    probe = {}
    for line in open(sys.argv[1], encoding="utf-8", errors="replace"):
        line = line.strip()
        if line.startswith("{"):
            row = json.loads(line)
            if row.get("class") == "Window":
                probe[str(row["id"])] = row
    model = ifcopenshell.open(sys.argv[2])
    scale = ifcopenshell.util.unit.calculate_unit_scale(model)
    settings = ifcopenshell.geom.settings()
    openings = {}
    for rel in model.by_type("IfcRelFillsElement"):
        openings[rel.RelatedBuildingElement.id()] = rel.RelatingOpeningElement
    groups = collections.defaultdict(list)
    for window in model.by_type("IfcWindow"):
        tag = str(window.Tag or "")
        row = probe.get(tag)
        if row is None:
            continue
        ow = round(float(window.OverallWidth or 0) * scale / FEET, 4)
        oh = round(float(window.OverallHeight or 0) * scale / FEET, 4)
        body = local_extents(settings, window)
        opening = openings.get(window.id())
        hole = local_extents(settings, opening) if opening is not None else None
        key = (
            row["type"],
            ow,
            oh,
            round(row["width"], 4) if row.get("width") is not None else None,
            round(row["height"], 4) if row.get("height") is not None else None,
            tuple(round(b, 4) for b in row["box"]),
            body,
            hole,
        )
        groups[key].append(tag)
    print(f"windows in both: {sum(len(v) for v in groups.values())}, distinct: {len(groups)}")
    for key, tags in sorted(groups.items(), key=lambda kv: -len(kv[1])):
        typ, ow, oh, w, h, box, body, hole = key
        print(f"{len(tags):4d} type {typ}: Revit {ow} x {oh}; type {w} x {h}; box {box}; body {body}; opening {hole}; e.g. {tags[0]}")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Score wall ends in an `rvt-gltf` GLB against a CoordinationView export.

Research tool for `reports/element-framing/RE-126-wall-flip-flag-and-2026-joins.md`.
A CoordinationView export (RE1's IFC2X3, the flowbim.ee house's IFC4X3)
writes each layered wall as one box: at a join, the wall that runs through
reaches its partner's far face and the one that stops ends at its near face,
with all its layers. rvt-rs draws the layers, and at a corner they nest
(RE-71), so comparing the union of our layers with Revit's box over-counts
differences that are the view's, not the decoder's.

For each wall end, this compares where Revit's body ends along the wall's
direction with where our layers end: the furthest layer end (what a
through wall's box shows) and the nearest (what a stopped wall's box
shows). An end counts as Revit's when it equals either within the
tolerance. A single-layer wall's two are the same.

Usage:

    python3 tools/re/wall_ends_cv_vs_ifc.py <model.glb> <revit-export.ifc> [--list N]

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.placement

import wall_layers_vs_ifc as layers

from ifc_common import report_mesh_error

TOLERANCES = (0.001, 0.01, 0.1)


def layer_ends(doc, blob):
    """ElementId -> list of each layer node's plan points, internal feet."""
    out = collections.defaultdict(list)
    for node in doc["nodes"]:
        extras = node.get("extras") or {}
        if extras.get("ifcType") != "IFCWALL" or "mesh" not in node:
            continue
        name = node.get("name") or ""
        digits = name[len(name.rstrip("0123456789")) :]
        if not digits:
            continue
        m4 = node["matrix"]
        prim = doc["meshes"][node["mesh"]]["primitives"][0]
        points = [
            (
                m4[0] * x + m4[4] * y + m4[8] * z + m4[12],
                m4[1] * x + m4[5] * y + m4[9] * z + m4[13],
            )
            for x, y, z in layers.accessor(doc, blob, prim["attributes"]["POSITION"])
        ]
        out[int(digits)].append(points)
    return out


def main():
    args = sys.argv[1:]
    if len(args) < 2:
        raise SystemExit(__doc__)
    listing = int(args[args.index("--list") + 1]) if "--list" in args else 0
    doc, blob = layers.read_glb(args[0])
    ours = layer_ends(doc, blob)
    f = ifcopenshell.open(args[1])
    to_internal, to_internal_direction = layers.site_to_internal(f)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    counts = collections.Counter()
    off = []
    for wall in f.by_type("IfcWall"):
        try:
            tag = int(wall.Tag)
        except (TypeError, ValueError):
            continue
        if tag not in ours:
            continue
        try:
            shape = ifcopenshell.geom.create_shape(settings, wall)
        except Exception as error:
            counts["IfcOpenShell cannot mesh the IFC wall"] += 1
            report_mesh_error(error)
            continue
        verts = shape.geometry.verts
        theirs = [to_internal(verts[i], verts[i + 1]) for i in range(0, len(verts), 3)]
        if not theirs:
            continue
        placement = ifcopenshell.util.placement.get_local_placement(wall.ObjectPlacement)
        d = to_internal_direction(placement[0][0], placement[1][0])
        if min(abs(d[0]), abs(d[1])) > 1e-6:
            counts["angled, not scored"] += 1
            continue
        along = lambda p: p[0] * d[0] + p[1] * d[1]
        revit = (min(map(along, theirs)), max(map(along, theirs)))
        if tag == 1348326:
            # Research trace for #1870 (not to land): each layer's ends along the
            # wall against Revit's, and its plan outline's corners.
            import math
            across = lambda p: -p[0] * d[1] + p[1] * d[0]
            print(f"trace {tag}: Revit along {revit[0]:.4f} {revit[1]:.4f}, across "
                  f"{min(map(across, theirs)):.4f} {max(map(across, theirs)):.4f}")
            for k, pts in enumerate(ours[tag]):
                uniq = sorted({(round(along(p), 4), round(across(p), 4)) for p in pts})
                print(f"  layer {k}: along {min(map(along, pts)):.4f} {max(map(along, pts)):.4f} "
                      f"across {min(map(across, pts)):.4f} {max(map(across, pts)):.4f} points {len(uniq)}")
                # The outline in plan: the hull of the layer's points, with each
                # corner's two edge lengths and its turn's sine.
                def hull(q):
                    q = sorted(set(q))
                    if len(q) < 3:
                        return q
                    def t(o, a, b):
                        return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
                    lo, up = [], []
                    for v in q:
                        while len(lo) >= 2 and t(lo[-2], lo[-1], v) <= 0: lo.pop()
                        lo.append(v)
                    for v in reversed(q):
                        while len(up) >= 2 and t(up[-2], up[-1], v) <= 0: up.pop()
                        up.append(v)
                    return lo[:-1] + up[:-1]
                ring = hull(uniq)
                for i, v in enumerate(ring):
                    a, b = ring[i - 1], ring[(i + 1) % len(ring)]
                    e1 = math.dist(a, v); e2 = math.dist(v, b)
                    cross = (v[0] - a[0]) * (b[1] - v[1]) - (v[1] - a[1]) * (b[0] - v[0])
                    sine = cross / (e1 * e2) if e1 and e2 else float("nan")
                    print(f"    corner {v[0]:.4f} {v[1]:.4f} edges {e1:.4f} {e2:.4f} sine {sine:.6f}")
        starts = [min(map(along, pts)) for pts in ours[tag]]
        ends = [max(map(along, pts)) for pts in ours[tag]]
        # At the start, "furthest" is the smallest value; at the end, the largest.
        candidates = ((min(starts), max(starts), revit[0]), (max(ends), min(ends), revit[1]))
        for slot, (furthest, nearest, r) in enumerate(candidates):
            error = min(abs(furthest - r), abs(nearest - r))
            counts["ends"] += 1
            for tolerance in TOLERANCES:
                if error <= tolerance:
                    counts[f"ends within {tolerance} ft"] += 1
            if error > TOLERANCES[0]:
                off.append((round(error, 4), tag, ("start", "end")[slot]))
    for key in sorted(counts):
        print(f"  {counts[key]:>5}  {key}")
    if listing:
        print("largest differences (feet, tag, end):")
        for row in sorted(off, reverse=True)[:listing]:
            print("  ", *row)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Compare each wall's saved mesh (probe_1474_wall_mesh_census) with Revit's body (#1474).

For every wall the probe prints, the IfcWall of the same Tag in Revit's export
is meshed (its openings subtracted, as Revit's body is written) and taken in
Revit's internal feet (ifc_common.site_frame). The saved mesh's volume and box
are compared with that body's, and the walls are counted by whether the mesh
closes (no open edge), how it decoded, and Revit's body representation type.
A mesh equals Revit's body here when its volume is within 0.1% and every
corner of its box within 0.01 ft.

Usage:

    python3 tools/re/wall_mesh_census_vs_ifc.py <probe output> <revit-export.ifc> [--list]
"""

import collections
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import ifcopenshell  # noqa: E402
import ifcopenshell.geom  # noqa: E402
from ifc_common import site_frame  # noqa: E402


def body_type(element):
    shape = element.Representation
    if shape is None:
        return "none"
    for rep in shape.Representations:
        if rep.RepresentationIdentifier == "Body":
            kind = rep.RepresentationType or "unset"
            if kind == "MappedRepresentation" and rep.Items:
                kind = "mapped " + (rep.Items[0].MappingSource.MappedRepresentation.RepresentationType or "unset")
            return kind
    return "no body"


def read_probe(path):
    walls = {}
    for line in open(path):
        p = line.split()
        if not p or p[0] != "wall":
            continue
        row = {"status": p[3], "diagnostics": int(p[5]), "unbounded": int(p[7]), "triangles": int(p[9])}
        if row["triangles"]:
            row["open"] = int(p[11])
            row["volume"] = float(p[13])
            row["box"] = np.array(list(map(float, p[15:21])))
        walls[p[1]] = row
    return walls


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    listing = "--list" in sys.argv
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    walls = read_probe(args[0])
    model = ifcopenshell.open(args[1])
    to_feet = site_frame(model)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    revit = {}
    for wall in model.by_type("IfcWall"):
        if wall.Tag:
            revit.setdefault(wall.Tag, wall)
    counts = collections.Counter()
    off = []
    for tag, row in sorted(walls.items()):
        theirs = revit.get(tag)
        if theirs is None:
            counts[("not in Revit's export", "-", "-")] += 1
            continue
        kind = body_type(theirs)
        if not row["triangles"]:
            counts[("no triangles", row["status"], kind)] += 1
            continue
        closed = "closed" if row["open"] == 0 else "open"
        try:
            shape = ifcopenshell.geom.create_shape(settings, theirs)
            geometry = shape.geometry
            verts = to_feet(np.array(geometry.verts, dtype=float).reshape(-1, 3))
            faces = np.array(geometry.faces, dtype=int).reshape(-1, 3)
        except Exception:  # noqa: BLE001 - a body IfcOpenShell cannot mesh is counted apart
            counts[(f"{closed}, Revit's body not meshed", row["status"], kind)] += 1
            continue
        a, b, c = verts[faces[:, 0]], verts[faces[:, 1]], verts[faces[:, 2]]
        volume = abs(np.einsum("ij,ij->i", a, np.cross(b, c)).sum() / 6.0)
        box = np.concatenate([verts.min(axis=0), verts.max(axis=0)])
        corner = float(np.abs(box - row["box"]).max())
        rel = abs(row["volume"] - volume) / volume if volume > 0 else float("inf")
        same = rel <= 1e-3 and corner <= 0.01
        verdict = "equal" if same else "different"
        counts[(f"{closed}, {verdict}", row["status"], kind)] += 1
        if not same:
            off.append((tag, closed, row["status"], kind, row["volume"], volume, corner, row["unbounded"]))
    print("walls in the probe:", len(walls))
    print("saved mesh against Revit's body (count | closure, verdict | decode status | Revit's body):")
    for (verdict, status, kind), n in sorted(counts.items(), key=lambda kv: (-kv[1], kv[0])):
        print(f"  {n:>5} | {verdict} | {status} | {kind}")
    if listing:
        for tag, closed, status, kind, ours, theirs, corner, unbounded in off[:60]:
            print(f"  wall {tag}: {closed} {status} unbounded {unbounded}, Revit {kind}, mesh {ours:.4f} ft3 vs {theirs:.4f}, box corner {corner:.4f} ft")
    return 0


if __name__ == "__main__":
    sys.exit(main())

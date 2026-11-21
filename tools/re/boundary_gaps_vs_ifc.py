#!/usr/bin/env python3
"""Where Revit's own body puts the boundary across each gap of a refused ring (#2085).

`probe_2085_trim_breaks` prints, for every element whose graphics reach a graph with a refused
plane or cylinder ring, each gap's two ends (`gapxyz ... p=x,y,z q=x,y,z`) and every edge end of
the same ring (`jointxyz ... at_gap=<bool> x=x,y,z`), in document feet. This reads Revit's body
for the same Tag (`mesh_bodies_vs_ifc.meshes`, Revit's internal feet) and prints, by counts and
distances only and never a coordinate:

- per Tag, the IFC items its body is made of: Revit's own vertices are compared only where the
  body is faceted (an `IfcFacetedBrep`, `IfcTriangulatedFaceSet` or `IfcPolygonalFaceSet`);
  elsewhere the vertices are IfcOpenShell's tessellation and the Tag is counted apart;
- the floor: for the ring ends that close (`at_gap=False`), the distance to Revit's nearest
  vertex, which says how closely Revit's vertices sit on the saved ring at all;
- per gap occurrence, with `tol` the larger of twice the Tag's floor (its median) and 0.00001
  ft: `unresolved` where the gap is under twice that floor; else Revit has a vertex within `tol`
  at P only, at Q only, at both, one strictly between them along the gap (within `tol` of the
  segment and farther than `tol` from both ends), or none of these;
- per gap (by graph, face, loop and position), the outcomes over its occurrences and whether
  they agree.

Needs IfcOpenShell and numpy.

    python3 tools/re/boundary_gaps_vs_ifc.py <probe output> <revit.ifc>
"""

import collections
import os
import re
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from mesh_bodies_vs_ifc import meshes  # noqa: E402

FACETED = {"IfcFacetedBrep", "IfcTriangulatedFaceSet", "IfcPolygonalFaceSet", "IfcShellBasedSurfaceModel"}
TOUCHING = 1e-5


def point(text):
    return np.array([float(x) for x in text.split(",")])


def fields(line):
    return dict(token.split("=", 1) for token in line.split()[1:] if "=" in token)


def read_probe(path):
    gaps, joints = [], []
    with open(path, errors="replace") as handle:
        for line in handle:
            if line.startswith("gapxyz "):
                f = fields(line)
                gaps.append(f)
            elif line.startswith("jointxyz "):
                joints.append(fields(line))
    return gaps, joints


def item_types(model, tags):
    out = collections.defaultdict(collections.Counter)
    for element in model.by_type("IfcElement"):
        if element.Tag not in tags or element.Representation is None:
            continue
        for rep in element.Representation.Representations:
            for item in rep.Items:
                kind = item.is_a()
                if kind == "IfcMappedItem":
                    for inner in item.MappingSource.MappedRepresentation.Items:
                        out[element.Tag][inner.is_a()] += 1
                else:
                    out[element.Tag][kind] += 1
    return out


def segment_distance(points, a, b):
    ab = b - a
    t = np.clip(((points - a) @ ab) / max(float(ab @ ab), 1e-24), 0.0, 1.0)
    return np.linalg.norm(points - (a + t[:, None] * ab), axis=1)


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        return 2
    import ifcopenshell

    gaps, joints = read_probe(sys.argv[1])
    tags = {g["owner"] for g in gaps}
    print(f"gap occurrences {len(gaps)} on {len(tags)} elements; ring ends {len(joints)}")
    if not gaps:
        print("not exercised: no gap reached by an element's graphics")
        return 0
    model = ifcopenshell.open(sys.argv[2])
    bodies = meshes(sys.argv[2], tags=tags, model=model)
    kinds = item_types(model, tags)
    faceted = {t for t, c in kinds.items() if c and set(c) <= FACETED}
    print(f"elements with a Revit body by Tag {len(set(bodies) & tags)} of {len(tags)}; faceted {len(faceted & set(bodies))}")
    item_census = collections.Counter(k for t in tags for k in kinds.get(t, {}))
    print("item types: " + ", ".join(f"{k} {v}" for k, v in sorted(item_census.items())))
    floors = {}
    for tag in sorted(set(bodies) & faceted):
        verts = np.unique(bodies[tag][2].reshape(-1, 3), axis=0)
        closed = [point(j["x"]) for j in joints if j["owner"] == tag and j["at_gap"] == "false"]
        if not closed:
            continue
        d = np.array([np.linalg.norm(verts - p, axis=1).min() for p in closed])
        floors[tag] = (float(np.median(d)), float(d.max()), len(d))
    if floors:
        med = sorted(v[0] for v in floors.values())
        print(f"floor (closed ring ends to Revit's nearest vertex), per-element median: min {med[0]:.9f} median {med[len(med)//2]:.9f} max {med[-1]:.9f} ft over {len(floors)} elements")
    outcomes = collections.Counter()
    per_gap = collections.defaultdict(collections.Counter)
    for g in gaps:
        tag = g["owner"]
        key = (g["graph"], g["face"], g["loop"], g["at"])
        if tag not in bodies:
            outcome = "no_revit_body"
        elif tag not in faceted:
            outcome = "not_faceted"
        elif tag not in floors:
            outcome = "no_floor"
        else:
            floor = floors[tag][0]
            tol = max(2 * floor, TOUCHING)
            gap = float(g["gap"])
            p, q = point(g["p"]), point(g["q"])
            verts = np.unique(bodies[tag][2].reshape(-1, 3), axis=0)
            if gap < 2 * tol:
                outcome = "unresolved"
            else:
                at_p = bool((np.linalg.norm(verts - p, axis=1) < tol).any())
                at_q = bool((np.linalg.norm(verts - q, axis=1) < tol).any())
                near_seg = segment_distance(verts, p, q) < tol
                away = (np.linalg.norm(verts - p, axis=1) > tol) & (np.linalg.norm(verts - q, axis=1) > tol)
                between = bool((near_seg & away).any())
                outcome = "between" if between else "both" if at_p and at_q else "p_only" if at_p else "q_only" if at_q else "none"
        outcomes[outcome] += 1
        per_gap[key][outcome] += 1
    print("occurrences by outcome: " + ", ".join(f"{k} {v}" for k, v in sorted(outcomes.items())))
    agree = sum(1 for c in per_gap.values() if len(c) == 1)
    print(f"gaps {len(per_gap)}; whose occurrences all agree {agree}")
    by_class = collections.Counter(max(c, key=c.get) for c in per_gap.values())
    print("gaps by their most common outcome: " + ", ".join(f"{k} {v}" for k, v in sorted(by_class.items())))
    for key, c in sorted(per_gap.items()):
        print(f"gap graph={key[0]} face={key[1]} loop={key[2]} at={key[3]} " + " ".join(f"{k}={v}" for k, v in sorted(c.items())))
    return 0


if __name__ == "__main__":
    sys.exit(main())

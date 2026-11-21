import os
"""#2111: rvt-rs's farthest vertex from Revit's surface over the vertices mesh_bodies_vs_ifc tests
(its own `sample`, at most VERTEX_CAP) and over every vertex, on base and on head.

    python sample2111.py <Revit ifc> <base ifc> <head ifc> <Tag>
"""
import functools, importlib.util, os, sys
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import place
from place import body, distance
place.ifcopenshell.open = functools.lru_cache(maxsize=4)(place.ifcopenshell.open)
spec = importlib.util.spec_from_file_location("mb", os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "tools", "re", "mesh_bodies_vs_ifc.py"))
mb = importlib.util.module_from_spec(spec); spec.loader.exec_module(mb)
revit, base, head, tag = sys.argv[1:5]
rv, rf = body(revit, tag); ra, rb, rc = rv[rf[:, 0]], rv[rf[:, 1]], rv[rf[:, 2]]
for name, path in (("base", base), ("head", head)):
    v, f = body(path, tag)
    tested = mb.sample(v[f])
    every = np.unique(np.round(v, 9), axis=0)
    far_t = max(distance(p, ra, rb, rc).min() for p in tested)
    far_e = max(distance(p, ra, rb, rc).min() for p in every)
    print(f"{tag} {name}: tested {len(tested)} of {len(every)} vertices; farthest tested {far_t:.6f} ft, farthest of all {far_e:.6f} ft")

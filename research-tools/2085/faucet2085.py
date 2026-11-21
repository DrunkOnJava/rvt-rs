import sys
tools, head, revit = sys.argv[1:4]
sys.path.insert(0, tools)
import mesh_bodies_vs_ifc as M, volume_readings as V
ours = M.meshes(head, tags={"1135955"}, source=M.SOURCE); ref = M.meshes(revit, tags=set(ours))
o, r = ours["1135955"][2], ref["1135955"][2]
far = max(M.surface_distance(M.sample(r), o), M.surface_distance(M.sample(o), r))
reading = V.read_body(o, r, far)
print("verdict:", V.verdict(reading))
import collections
print("shells by kind:", collections.Counter(x[0] for x in reading["shells"]))
for x in reading["shells"]:
    if x[0] != "equal": print("  ", x[:4])
import numpy as np
def open_edges(t):
    c = collections.Counter()
    for tri in np.round(t, 7):
        pts = [tuple(p) for p in tri]
        for a, b in ((0, 1), (1, 2), (2, 0)):
            c[tuple(sorted((pts[a], pts[b])))] += 1
    return sum(1 for v in c.values() if v == 1)
print("rvt-rs mesh: triangles", len(o), "edges held by one triangle", open_edges(o))
print("Revit mesh: triangles", len(r), "edges held by one triangle", open_edges(r))

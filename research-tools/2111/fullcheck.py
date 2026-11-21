"""#2111: every body whose mesh differs between base and head, read with every vertex against
Revit's export: Revit's vertices beyond 0.001 and 0.01 ft of rvt-rs's surface, and rvt-rs's farthest
vertex from Revit's surface, on each side. Prints totals and every body worse on head.

    python fullcheck.py <Revit ifc> <base ifc> <head ifc>
"""
import functools, importlib.util, os, sys
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import place
from place import body, distance
place.ifcopenshell.open = functools.lru_cache(maxsize=4)(place.ifcopenshell.open)
spec = importlib.util.spec_from_file_location("bd", os.path.join(os.path.dirname(os.path.abspath(__file__)), "ifc_body_diff.py"))
bd = importlib.util.module_from_spec(spec); spec.loader.exec_module(bd)
revit, base, head = sys.argv[1:4]
pa, pb = bd.read(base), bd.read(head)
moved = sorted({str(pb[g][1]) for g in set(pa) & set(pb) if pa[g][4] != pb[g][4] or pa[g][0] != pb[g][0]})
def measures(rv, rf, v, f):
    ta, tb, tc = v[f[:, 0]], v[f[:, 1]], v[f[:, 2]]
    ra, rb, rc = rv[rf[:, 0]], rv[rf[:, 1]], rv[rf[:, 2]]
    rp = np.unique(np.round(rv[np.unique(rf)], 6), axis=0)
    op = np.unique(np.round(v[np.unique(f)], 6), axis=0)
    d_r = np.array([distance(p, ta, tb, tc).min() for p in rp])
    d_o = np.array([distance(p, ra, rb, rc).min() for p in op])
    return int((d_r > 0.001).sum()), int((d_r > 0.01).sum()), float(d_o.max()) if len(d_o) else 0.0
tot = {"base": [0, 0], "head": [0, 0]}; worse = []; read = 0
for tag in moved:
    try:
        rv, rf = body(revit, tag); bv, bf = body(base, tag); hv, hf = body(head, tag)
    except StopIteration:
        continue
    read += 1
    b, h = measures(rv, rf, bv, bf), measures(rv, rf, hv, hf)
    for k, m in (("base", b), ("head", h)):
        tot[k][0] += m[0]; tot[k][1] += m[1]
    if h[0] > b[0] or h[1] > b[1] or h[2] > b[2] + 1e-6:
        worse.append((tag, b, h))
print(f"moved {len(moved)}, in Revit's export {read}")
for k, (a, c) in tot.items(): print(f"  {k}: Revit's vertices beyond 0.001 ft {a}, beyond 0.01 ft {c}")
print(f"  worse on head by any count: {len(worse)}")
for tag, b, h in worse[:30]: print(f"    {tag}: beyond 0.001 {b[0]}->{h[0]}, beyond 0.01 {b[1]}->{h[1]}, rvt-rs farthest {b[2]:.5f}->{h[2]:.5f}")

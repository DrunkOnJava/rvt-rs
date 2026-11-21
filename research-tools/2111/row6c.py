"""#2111 row 6: over the bodies whose mesh differs between exports A and B (ifc_body_diff.read),
Revit's vertices beyond 0.001 ft and 0.01 ft of rvt-rs's surface on each of several exports.

    python row6c.py <Revit ifc> <A ifc> <B ifc> <label=ifc> [<label=ifc> ...]
"""
import functools, importlib.util, os, sys
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import place
from place import body, distance
place.ifcopenshell.open = functools.lru_cache(maxsize=6)(place.ifcopenshell.open)
spec = importlib.util.spec_from_file_location("bd", os.path.join(os.path.dirname(os.path.abspath(__file__)), "ifc_body_diff.py"))
bd = importlib.util.module_from_spec(spec); spec.loader.exec_module(bd)
revit, a, b = sys.argv[1:4]
sides = [x.split("=", 1) for x in sys.argv[4:]]
pa, pb = bd.read(a), bd.read(b)
moved = sorted(str(pb[g][1]) for g in set(pa) & set(pb) if pa[g][4] != pb[g][4] or pa[g][0] != pb[g][0])
print("bodies moved between A and B:", len(moved))
totals = {label: [0, 0] for label, _ in sides}
rev_tags = set()
for tag in moved:
    try:
        rv, rf = body(revit, tag)
    except StopIteration:
        continue
    rev_tags.add(tag)
    pts = np.unique(np.round(rv[np.unique(rf)], 6), axis=0)
    for label, path in sides:
        try:
            v, f = body(path, tag)
        except StopIteration:
            continue
        ta, tb, tc = v[f[:, 0]], v[f[:, 1]], v[f[:, 2]]
        d = np.array([distance(p, ta, tb, tc).min() for p in pts])
        totals[label][0] += int((d > 0.001).sum()); totals[label][1] += int((d > 0.01).sum())
print("bodies in Revit's export:", len(rev_tags))
for label, (n1, n2) in totals.items():
    print(f"{label}: Revit vertices beyond 0.001 ft {n1}, beyond 0.01 ft {n2}")

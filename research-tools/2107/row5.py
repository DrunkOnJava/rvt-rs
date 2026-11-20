"""Row 5 of #2107: every body that moves on Snowdon Towers Architectural, read against Revit's export.

    python row5.py <Revit ifc> <base ifc> <head ifc>

The moved bodies are those whose mesh differs between base and head (ifc_body_diff.read). For
each, both directions (pair.py's `far`, in place.body's far_vertices frame, feet) on base and on
head: Revit's vertices beyond 0.001 and 0.01 ft of our surface, and ours beyond those of Revit's.
Prints totals, by family, and every body worse on head: more of Revit's vertices beyond 0.001 or 0.01 ft
of rvt-rs's surface (Revit's vertices are fixed, so their count is the measure), or rvt-rs's farthest
vertex from Revit's surface farther (rvt-rs's vertex count moves with its own mesh, so its count is not).
"""
import collections
import functools
import importlib.util
import sys

import numpy as np

sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))
import place  # noqa: E402
from place import body, distance  # noqa: E402

place.ifcopenshell.open = functools.lru_cache(maxsize=4)(place.ifcopenshell.open)
spec = importlib.util.spec_from_file_location(
    "bd", __import__("os").path.join(__import__("os").path.dirname(__import__("os").path.abspath(__file__)), "ifc_body_diff.py"))
bd = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bd)


def far(points, verts, faces):
    a, b, c = verts[faces[:, 0]], verts[faces[:, 1]], verts[faces[:, 2]]
    pts = np.unique(np.round(points, 6), axis=0)
    d = np.array([float(distance(p, a, b, c).min()) for p in pts])
    return float(d.max()), int((d > 0.001).sum()), int((d > 0.01).sum())


def both(revit, ours, tag):
    rv, rf = body(revit, tag)
    ov, of = body(ours, tag)
    r = far(rv[np.unique(rf)], ov, of)
    o = far(ov[np.unique(of)], rv, rf)
    return r, o


revit, base, head = sys.argv[1:4]
bp, hp = bd.read(base), bd.read(head)
moved = sorted({str(hp[g][1]) for g in hp if g in bp and hp[g][4] != bp[g][4]})
names = {str(hp[g][1]): (hp[g][0], g) for g in hp}
print(f"moved {len(moved)}")
totals = collections.Counter()
worse, missing = [], []
families = collections.Counter()
for tag in moved:
    try:
        rb, ob = both(revit, base, tag)
        rh, oh = both(revit, head, tag)
    except StopIteration:
        missing.append(tag)
        continue
    e = place.ifcopenshell.open(head).by_guid(names[tag][1])
    families[(names[tag][0], (e.Name or "").rsplit(":", 1)[0])] += 1
    for k, x, y in (("revit>0.001", rb[1], rh[1]), ("revit>0.01", rb[2], rh[2]),
                    ("ours>0.001", ob[1], oh[1]), ("ours>0.01", ob[2], oh[2])):
        totals[k + " base"] += x
        totals[k + " head"] += y
    if rh[1] > rb[1] or rh[2] > rb[2] or oh[0] > ob[0] + 1e-6:
        worse.append((tag, rb, rh, ob, oh))
print("not in Revit's export:", len(missing), missing[:10])
for k, n in families.most_common():
    print(f"  {n:4} {k[0]} {k[1]!r}")
for k in ("revit>0.001", "revit>0.01", "ours>0.001", "ours>0.01"):
    print(f"{k:12} base {totals[k + ' base']:7} head {totals[k + ' head']:7}")
print(f"bodies worse on head: {len(worse)}")
for w in worse:
    print(f"  {w[0]}: Revit's off ours base {w[1]} head {w[2]} | ours off Revit's base {w[3]} head {w[4]}")

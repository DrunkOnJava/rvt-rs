"""Per-axis gaps between a newly drawn body and its old record box, with each object's name.

    python box_axes.py <base ifc> <head ifc> [<class> <how many>]

For the moved objects of the class (all classes if none), the largest gaps first: the object's
Name, the box's three extents, and per box axis the gap at its low and high side (metres).
"""
import importlib.util, os, sys
import ifcopenshell, ifcopenshell.geom
import numpy as np
spec = importlib.util.spec_from_file_location("box_check", os.path.join(os.path.dirname(os.path.abspath(__file__)), "box_check.py"))
src = open(spec.origin).read().replace("\nmain()\n", "\n")
ns = {"__file__": spec.origin}
exec(compile(src, spec.origin, "exec"), ns)
base_path, head_path = sys.argv[1], sys.argv[2]
want = sys.argv[3].upper() if len(sys.argv) > 3 else None
limit = int(sys.argv[4]) if len(sys.argv) > 4 else 5
bp, hp = ns["body_diff"].read(base_path), ns["body_diff"].read(head_path)
moved = [g for g, h in hp.items() if "'saved_graphics_mesh'" in h[2] and g in bp and any(f"'{b}'" in bp[g][2] for b in ns["BOXES"])]
base, head = ifcopenshell.open(base_path), ifcopenshell.open(head_path)
st = ifcopenshell.geom.settings(); st.set("use-world-coords", True)
rows = []
for g in moved:
    if want and hp[g][0] != want: continue
    bv, bf = ns["mesh"](st, base.by_guid(g)); hv, _ = ns["mesh"](st, head.by_guid(g))
    axes = ns["box_axes"](bv, bf)
    if len(axes) != 3: continue
    per = []
    for a in axes:
        b, h = bv @ a, hv @ a
        per.append((round(float(np.ptp(b)), 4), round(float(h.min() - b.min()), 4), round(float(b.max() - h.max()), 4)))
    gap = max(max(x[1], x[2]) for x in per)
    rows.append((gap, hp[g][0], hp[g][1], head.by_guid(g).Name, per))
for gap, kind, tag, name, per in sorted(rows, reverse=True)[:limit]:
    print(f"{kind} {tag} {name!r} gap {gap:.4f}: per axis (extent, low gap, high gap) {per}")
names = sorted({r[3] for r in rows})
print(f"{len(rows)} read; names: {names[:12]}")

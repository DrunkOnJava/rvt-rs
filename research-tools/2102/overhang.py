"""Where a newly drawn body reaches past its old record box: which side, by how much, and which shell.

    python overhang.py <base ifc> <head ifc> <Tag> ...
"""
import collections, importlib.util, os, sys
import ifcopenshell, ifcopenshell.geom
import numpy as np
src = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "box_check.py")).read().replace("\nmain()\n", "\n")
ns = {"__file__": os.path.join(os.path.dirname(os.path.abspath(__file__)), "box_check.py")}
exec(compile(src, ns["__file__"], "exec"), ns)
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from legs import shells
base, head = ifcopenshell.open(sys.argv[1]), ifcopenshell.open(sys.argv[2])
st = ifcopenshell.geom.settings(); st.set("use-world-coords", True)
for tag in sys.argv[3:]:
    b = next(p for p in base.by_type("IfcProduct") if str(getattr(p, "Tag", "")) == tag and p.Representation and not p.is_a("IfcOpeningElement"))
    h = head.by_guid(b.GlobalId)
    bv, bf = ns["mesh"](st, b); hv, hf = ns["mesh"](st, h)
    axes = ns["box_axes"](bv, bf)
    owner = np.full(len(hv), -1); desc = []
    for k, tris in enumerate(sorted(shells(hv, hf), key=len, reverse=True)):
        owner[hf[tris].ravel()] = k
        pts = hv[hf[tris]].reshape(-1, 3); c = pts - pts.mean(axis=0)
        _, _, ax = np.linalg.svd(c, full_matrices=False)
        desc.append(sorted((round(float(np.ptp(c @ a)), 4) for a in ax), reverse=True))
    print(f"== {tag} {h.Name}")
    for k, a in enumerate(axes):
        bp, hp = bv @ a, hv @ a
        for side, beyond in (("low", bp.min() - hp), ("high", hp - bp.max())):
            far = beyond > 0.0002
            if far.any():
                shells_hit = collections.Counter(int(owner[i]) for i in np.where(far)[0])
                print(f"  axis {k} (box extent {np.ptp(bp):.4f}) {side} side: {int(far.sum())} vertices past, largest {beyond.max()*1000:.2f} mm, shells {dict(shells_hit)}")
    for k, d in enumerate(desc[:9]):
        print(f"  shell {k} extents {d}")

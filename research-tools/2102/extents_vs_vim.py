import os
"""Whole-body extents (along each body's own principal axes, and z) in rvt-rs's export and the VIM.

    python extents_vs_vim.py <ifc> <vim> <document> <Tag> ...
"""
import importlib.util, sys
import numpy as np
spec = importlib.util.spec_from_file_location("fm", os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "tools", "re", "footing_meshes_vs_vim.py"))
fm = importlib.util.module_from_spec(spec); spec.loader.exec_module(fm)
import ifcopenshell, ifcopenshell.geom
model = ifcopenshell.open(sys.argv[1]); vim = fm.load_vim(sys.argv[2], sys.argv[3])
st = ifcopenshell.geom.settings(); st.set("use-world-coords", True)
for tag in sys.argv[4:]:
    product = next(p for p in model.by_type("IfcProduct") if str(p.Tag) == tag and p.Representation)
    out = []
    for tris in (fm.ifc_triangles(product, st), fm.vim_triangles(vim, int(tag))):
        pts = np.asarray(tris, dtype=float).reshape(-1, 3)
        c = pts - pts.mean(axis=0)
        _, _, ax = np.linalg.svd(c, full_matrices=False)
        out.append((sorted((round(float(np.ptp(c @ a)), 4) for a in ax), reverse=True), round(float(np.ptp(pts[:, 2])), 4)))
    print(f"{tag} {product.Name}: rvt-rs extents {out[0][0]} height {out[0][1]} | VIM extents {out[1][0]} height {out[1][1]}")

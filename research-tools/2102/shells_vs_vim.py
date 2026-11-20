import os
"""One element's shells in rvt-rs's export and in the VIM: triangles, open edges, signed volume, extents.

    python shells_vs_vim.py <ifc> <vim> <document> <Tag> ...
"""
import collections, importlib.util, sys
import numpy as np
spec = importlib.util.spec_from_file_location("fm", os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "tools", "re", "footing_meshes_vs_vim.py"))
fm = importlib.util.module_from_spec(spec); spec.loader.exec_module(fm)
import ifcopenshell, ifcopenshell.geom


def shells(tris):
    key = {}
    ids = np.array([[key.setdefault(tuple(np.round(p, 6)), len(key)) for p in t] for t in tris])
    parent = list(range(len(key)))
    def find(i):
        while parent[i] != i:
            parent[i] = parent[parent[i]]; i = parent[i]
        return i
    for a, b, c in ids:
        for x, y in ((a, b), (b, c)):
            ra, rb = find(x), find(y)
            if ra != rb: parent[ra] = rb
    groups = collections.defaultdict(list)
    for t, (a, b, c) in enumerate(ids): groups[find(a)].append(t)
    out = []
    for g in groups.values():
        edges = collections.Counter()
        for t in g:
            a, b, c = ids[t]
            for e in ((a, b), (b, c), (c, a)): edges[frozenset(e)] += 1
        p = tris[g]
        vol = float(np.einsum("ij,ij->i", p[:, 0], np.cross(p[:, 1], p[:, 2])).sum() / 6.0)
        pts = p.reshape(-1, 3); c = pts - pts.mean(axis=0)
        _, _, ax = np.linalg.svd(c, full_matrices=False)
        ext = sorted((round(float(np.ptp(c @ a)), 3) for a in ax), reverse=True)
        out.append((abs(vol), vol, len(g), sum(1 for n in edges.values() if n == 1), ext))
    return sorted(out, reverse=True)


model = ifcopenshell.open(sys.argv[1]); vim = fm.load_vim(sys.argv[2], sys.argv[3])
st = ifcopenshell.geom.settings(); st.set("use-world-coords", True)
for tag in sys.argv[4:]:
    product = next(p for p in model.by_type("IfcProduct") if str(p.Tag) == tag and p.Representation)
    print(f"== {tag} {product.Name}")
    for name, tris in (("rvt-rs", fm.ifc_triangles(product, st)), ("VIM", fm.vim_triangles(vim, int(tag)))):
        rows = shells(np.asarray(tris, dtype=float))
        total = sum(r[1] for r in rows)
        print(f"  {name}: {len(rows)} shells, signed volume {total:.6f} m3")
        for _, vol, n, opened, ext in rows[:12]:
            print(f"    shell vol={vol:.6f} tris={n} open_edges={opened} extents_m={ext}")

#!/usr/bin/env python3
"""Which coverings Revit's export gives a `GrossArea`, and what each body's faces measure (#1209).

For every `IfcCovering` both files hold under one `Tag`, this pairs rvt-rs's
covering with Revit's and counts, by rvt-rs's `RevitClass` (its
`RvtElementRecordGeometry`-style class property) and Revit's
`PredefinedType`:

- how many carry Revit's `Qto_CoveringBaseQuantities.GrossArea`, and how many
  carry rvt-rs's;
- of those Revit gives one, how many equal, within 1e-3 relative, the area of
  the largest downward horizontal face of rvt-rs's body, of its largest upward
  one, of its plan box (x extent times y extent in world coordinates), and of
  the largest downward face of Revit's own body;
- the bodies' extents, so that a cube, whose every face has one area, can be
  told apart.

Areas in square feet, lengths in feet, from IfcOpenShell's meshes in world
coordinates. Lines starting `covering:` list one non-ceiling covering each and
name its Tag, so a held-out model's are never kept.

Usage:

    python3 tools/re/covering_areas_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>
"""

import collections
import os
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element as ue
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import qtos_vs_ifc  # noqa: E402
import wall_heights_vs_ifc as base  # noqa: E402

FOOT = 0.3048
SET = "Qto_CoveringBaseQuantities"
TOL = 1e-3


def faces(path, product, settings):
    """(largest downward face, largest upward face, x, y, z extents) in feet."""
    geometry = base.body_geometry(settings, product, path)
    if geometry is None or not len(geometry.verts):
        return None
    v = np.asarray(geometry.verts, dtype=float).reshape(-1, 3) / FOOT
    t = np.asarray(geometry.faces, dtype=int).reshape(-1, 3)
    a, b, c = v[t[:, 0]], v[t[:, 1]], v[t[:, 2]]
    n = np.cross(b - a, c - a)
    length = np.linalg.norm(n, axis=1)
    live = length > 1e-12
    nz = np.zeros(len(t))
    nz[live] = n[live, 2] / length[live]
    levels = collections.defaultdict(float)
    members = collections.defaultdict(list)
    for i in np.where(live & (np.abs(nz) >= 0.999))[0]:
        key = (round(float(a[i, 2]), 3), bool(nz[i] > 0))
        levels[key] += float(length[i]) / 2
        members[key].append(int(i))
    down_key = max((k for k in levels if not k[1]), key=lambda k: levels[k], default=None)
    up_key = max((k for k in levels if k[1]), key=lambda k: levels[k], default=None)
    down = levels[down_key] if down_key else None
    up = levels[up_key] if up_key else None
    span = v.max(axis=0) - v.min(axis=0)
    # Connected parts: of the whole mesh, and of each largest face, joining
    # triangles that share a corner 1e-5 ft apart.
    corner = {}
    for i, row in enumerate(t):
        for j in row:
            corner.setdefault(tuple(np.round(v[j] / 1e-5).astype(np.int64)), len(corner))
    ids = np.array([[corner[tuple(np.round(v[j] / 1e-5).astype(np.int64))] for j in row] for row in t])

    def parts(rows):
        parent = {}

        def find(x):
            while parent.setdefault(x, x) != x:
                parent[x] = parent[parent[x]]
                x = parent[x]
            return x

        for r in rows:
            p0, p1, p2 = (find(int(x)) for x in ids[r])
            parent[p1] = p0
            parent[p2] = p0
        return len({find(int(ids[r][0])) for r in rows})

    solids = parts(range(len(t)))
    down_parts = parts(members[down_key]) if down_key else 0
    up_parts = parts(members[up_key]) if up_key else 0
    return down, up, float(span[0]), float(span[1]), float(span[2]), solids, down_parts, up_parts


def collect(path):
    f = qtos_vs_ifc.open_ifc(path)
    quantities = qtos_vs_ifc.quantities(path)[0]
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    out = {}
    for product in f.by_type("IfcCovering"):
        if not product.Tag or str(product.Tag) in out:
            continue
        tag = str(product.Tag)
        psets = ue.get_psets(product)
        cls = next((str(p["RevitClass"]) for p in psets.values() if "RevitClass" in p), "-")
        source = next((str(p["BodySource"]) for p in psets.values() if "BodySource" in p), "-")
        area = quantities.get((f"tag:{tag}", SET, "GrossArea"))
        out[tag] = {
            "class": cls,
            "type": str(product.PredefinedType),
            "object": str(product.ObjectType),
            "source": source,
            "area": None if area is None or area[1] is None else area[1] / FOOT**2,
            "faces": faces(path, product, settings) if product.Representation else None,
        }
    return out


def near(x, y):
    return x is not None and y is not None and abs(x - y) <= TOL * max(abs(y), 1e-9)


def main():
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    ours, revit = collect(sys.argv[1]), collect(sys.argv[2])
    both = sorted(set(ours) & set(revit))
    print(f"coverings: rvt-rs {len(ours)}, Revit {len(revit)}, both {len(both)}")
    tally = collections.defaultdict(collections.Counter)
    for tag in both:
        u, r = ours[tag], revit[tag]
        key = (u["class"], r["type"])
        row = tally[key]
        row["paired"] += 1
        row["Revit has GrossArea"] += r["area"] is not None
        row["rvt-rs has GrossArea"] += u["area"] is not None
        if r["area"] is None:
            continue
        row["rvt-rs equal"] += near(u["area"], r["area"])
        uf, rf = u["faces"], r["faces"]
        if uf is not None:
            row["rvt-rs lower face equal"] += near(uf[0], r["area"])
            row["rvt-rs upper face equal"] += near(uf[1], r["area"])
            row["rvt-rs plan box equal"] += near(uf[2] * uf[3], r["area"])
            dims = sorted(round(d, 3) for d in uf[2:])
            row["rvt-rs body a cube"] += dims[0] == dims[2]
            one = uf[5] == 1 and uf[6] == 1 and uf[7] == 1 and near(uf[0], uf[1])
            row["rvt-rs one part, one-piece faces agreeing"] += one
            row["of those, lower face equal"] += one and near(uf[0], r["area"])
        if rf is not None:
            row["Revit lower face equal"] += near(rf[0], r["area"])
    for (cls, kind), row in sorted(tally.items()):
        print(f"coverings: class {cls} type {kind}: " + ", ".join(f"{k} {v}" for k, v in row.items()))
    for tag in both:
        u, r = ours[tag], revit[tag]
        if u["class"] == "Ceiling":
            continue
        uf, rf = u["faces"], r["faces"]
        def fmt(x):
            return "-" if x is None else f"{x:.4f}"
        print(
            f"covering: tag {tag} class {u['class']} type {r['type']} object {r['object']!r} source {u['source']} "
            f"Revit {fmt(r['area'])} rvt-rs {fmt(u['area'])} | rvt-rs lower {fmt(uf and uf[0])} upper {fmt(uf and uf[1])} "
            f"box {fmt(uf and uf[2] * uf[3])} span {fmt(uf and uf[2])} {fmt(uf and uf[3])} {fmt(uf and uf[4])} | "
            f"Revit lower {fmt(rf and rf[0])} span {fmt(rf and rf[2])} {fmt(rf and rf[3])} {fmt(rf and rf[4])} | "
            f"rvt-rs parts {uf and uf[5]} lower pieces {uf and uf[6]} upper pieces {uf and uf[7]}; Revit parts {rf and rf[5]}"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())

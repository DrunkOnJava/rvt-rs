#!/usr/bin/env python3
"""Which reading of a HermiteSurf's mixed derivatives puts its patch on Revit's own mesh (#1191, RE-460).

`probes/examples/probe_re460_hermite_surf.rs` stage 3 prints the nodes of the Hermite surfaces of an
element: the points, the tangents in u and in v and the mixed derivative `w` (`m_iMixedDer`) at each
crossing of the grid. A bicubic Hermite patch on a cell does not depend on `w` along the cell's edges, so
the edge samples of stage 2, which the patch met to 0.001 ft, did not test how `w` is scaled. A vertex of
Revit's own mesh of the same element lies on the same surface, in the middle of a cell as well as on its
edge, so it does.

For each surface of the probe's output, this evaluates the patch of every cell, densely, with the nodes'
points and tangents as stage 2 reads them (the tangent times the cell's width) and with `w` read in
several ways, and counts how many of the vertices of Revit's body (the elements whose `Tag` the probe's
`instance` lines give for the surface's symbol, in the internal frame) that lie within 0.2 ft of the
surface lie within 0.01 ft of it:

    python3 tools/re/hermite_twist_vs_ifc.py <probe.txt> <revit.ifc>

Needs IfcOpenShell and numpy.
"""

import collections
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mesh_bodies_vs_ifc as mesh  # noqa: E402

NEAR = 0.2  # feet: a vertex this close to the surface is taken to be on it
BAR = 0.01  # feet
SPACING = 0.01  # feet: the density of the samples of the patch


def read_surfaces(path):
    """((symbol, k) -> {u, v, nodes[(iu, iv)] -> (p, tu, tv, w)}, symbol -> the Tags of the elements that draw it)."""
    out = {}
    users = collections.defaultdict(list)
    for line in open(path, encoding="utf-8", errors="replace"):
        parts = line.split()
        if not parts:
            continue
        if parts[0] == "instance" and len(parts) == 4 and parts[2] == "symbol":
            users[parts[3]].append(parts[1])
        elif parts[0] == "surface" and len(parts) > 3 and parts[3] == "nu":
            key = (parts[1], int(parts[2]))
            out[key] = {
                "u": np.array([float(x) for x in parts[8].split(",")]),
                "v": np.array([float(x) for x in parts[10].split(",")]),
                "nodes": {},
            }
        elif parts[0] == "node":
            key = (parts[1], int(parts[2]))
            if key not in out:
                continue
            # parts: node id k iu iv p x y z tu x y z tv x y z w x y z
            numbers = [float(x) for x in parts[5:] if x not in ("p", "tu", "tv", "w")]
            p, tu, tv, w = (np.array(numbers[i : i + 3]) for i in (0, 3, 6, 9))
            out[key]["nodes"][(int(parts[3]), int(parts[4]))] = (p, tu, tv, w)
    return out, users


def basis(t):
    h = {
        "v0": 2 * t**3 - 3 * t**2 + 1,
        "v1": -2 * t**3 + 3 * t**2,
        "s0": t**3 - 2 * t**2 + t,
        "s1": t**3 - t**2,
    }
    return h


def patch_points(surface, twist):
    """Points of every cell's patch, `twist(du, dv)` scaling the mixed derivative of a cell."""
    u, v, nodes = surface["u"], surface["v"], surface["nodes"]
    out = []
    for j in range(len(v) - 1):
        for i in range(len(u) - 1):
            du, dv = u[i + 1] - u[i], v[j + 1] - v[j]
            corner = [nodes[(i + a, j + b)] for b in (0, 1) for a in (0, 1)]
            length = max(
                np.linalg.norm(corner[1][0] - corner[0][0]),
                np.linalg.norm(corner[2][0] - corner[0][0]),
                np.linalg.norm(corner[3][0] - corner[0][0]),
            )
            n = int(min(max(np.ceil(length / SPACING), 8), 160))
            t = np.linspace(0.0, 1.0, n)
            tt, ss = np.meshgrid(t, t, indexing="ij")
            bt, bs = basis(tt), basis(ss)
            point = np.zeros(tt.shape + (3,))
            for b in (0, 1):
                for a in (0, 1):
                    p, tu, tv, w = nodes[(i + a, j + b)]
                    vu, su = bt[f"v{a}"], bt[f"s{a}"]
                    vv, sv = bs[f"v{b}"], bs[f"s{b}"]
                    point += (vu * vv)[..., None] * p
                    point += (su * vv)[..., None] * (tu * du)
                    point += (vu * sv)[..., None] * (tv * dv)
                    point += (su * sv)[..., None] * (w * twist(du, dv))
            out.append(point.reshape(-1, 3))
    return np.concatenate(out)


def nearest_distance(cloud, queries, cell=NEAR):
    """The distance from each query to its nearest point of `cloud`, exact where it is at most `cell`.

    The cloud is binned in cubes of `cell` feet, so any point within `cell` of a query is in the query's
    cube or one of its 26 neighbours; a query with none of them near gets a larger distance or infinity.
    numpy alone: the runner's pinned requirements have no scipy.
    """
    keys = np.floor(cloud / cell).astype(np.int64)
    order = np.lexsort((keys[:, 2], keys[:, 1], keys[:, 0]))
    cloud, keys = cloud[order], keys[order]
    starts = np.concatenate(([0], np.nonzero(np.any(keys[1:] != keys[:-1], axis=1))[0] + 1))
    ends = np.concatenate((starts[1:], [len(cloud)]))
    cubes = {tuple(int(c) for c in keys[s]): (s, e) for s, e in zip(starts, ends)}
    around = [(a, b, c) for a in (-1, 0, 1) for b in (-1, 0, 1) for c in (-1, 0, 1)]
    out = np.full(len(queries), np.inf)
    for index, query in enumerate(queries):
        x, y, z = (int(c) for c in np.floor(query / cell))
        near = [cloud[s:e] for s, e in (cubes.get((x + a, y + b, z + c), (0, 0)) for a, b, c in around) if e > s]
        if near:
            out[index] = float(np.sqrt(((np.concatenate(near) - query) ** 2).sum(axis=1).min()))
    return out


CONVENTIONS = collections.OrderedDict(
    [
        ("w times du*dv (the cell's area)", lambda du, dv: du * dv),
        ("minus w times du*dv", lambda du, dv: -du * dv),
        ("no twist", lambda du, dv: 0.0),
        ("w as stored, unscaled", lambda du, dv: 1.0),
        ("w times du", lambda du, dv: du),
        ("w times dv", lambda du, dv: dv),
        ("w times du*dv, doubled", lambda du, dv: 2 * du * dv),
        ("w times du*dv, halved", lambda du, dv: 0.5 * du * dv),
        ("w times du*dv, quartered", lambda du, dv: 0.25 * du * dv),
    ]
)


def main():
    if len(sys.argv) != 3:
        print(__doc__)
        return 2
    surfaces, users = read_surfaces(sys.argv[1])
    if not surfaces:
        print("hermite twist: no surface in the probe output")
        return 0
    # A surface is on its symbol's record; Revit's export has the elements that place the symbol.
    tags = {symbol: users.get(symbol) or [symbol] for symbol, _ in surfaces}
    revit = mesh.meshes(sys.argv[2], tags={tag for named in tags.values() for tag in named})
    for (symbol, k), surface in sorted(surfaces.items()):
        found = [tag for tag in tags[symbol] if tag in revit]
        if not found:
            print(f"hermite twist: symbol {symbol} surface {k}: no element of Revit's export with Tag {', '.join(tags[symbol])}")
            continue
        vertices = np.unique(np.round(np.concatenate([revit[tag][2].reshape(-1, 3) for tag in found]), 6), axis=0)
        cells = (len(surface["u"]) - 1) * (len(surface["v"]) - 1)
        print(f"hermite twist: symbol {symbol} (elements {', '.join(found)}) surface {k}: {len(surface['u'])} by {len(surface['v'])} nodes, {cells} cells, Revit's bodies {len(vertices)} vertices")
        for name, twist in CONVENTIONS.items():
            cloud = patch_points(surface, twist)
            distance = nearest_distance(cloud, vertices)
            near = distance <= NEAR
            held = int((distance <= BAR).sum())
            print(f"hermite twist:   {name}: of the {int(near.sum())} vertices within {NEAR} ft of the surface, within {BAR} ft: {held}; median {float(np.median(distance[near])) if near.any() else float('nan'):.4f} ft")
    return 0


if __name__ == "__main__":
    sys.exit(main())

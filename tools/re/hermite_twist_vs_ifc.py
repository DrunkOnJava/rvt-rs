#!/usr/bin/env python3
"""Which reading of a HermiteSurf's mixed derivatives puts its patch on Revit's own mesh (#1191, RE-460).

`probes/examples/probe_re460_hermite_surf.rs` stage 3 prints the nodes of the Hermite surfaces of an
element: the points, the tangents in u and in v and the mixed derivative `w` (`m_iMixedDer`) at each
crossing of the grid. A bicubic Hermite patch on a cell does not depend on `w` along the cell's edges, so
the edge samples of stage 2, which the patch met to 0.001 ft, did not test how `w` is scaled. A vertex of
Revit's own mesh of the same element lies on the same surface, in the middle of a cell as well as on its
edge, so it does.

The nodes are the symbol's own coordinates. The probe's `instance` line names the element that places
the symbol (the `Tag` of Revit's export) and, where its `m_Trf` was read, the rows of `m_3x3` and `m_or`
that place it (the rows applied to the coordinates, then `m_or` added, RE-450); a line without them
places the symbol at the origin, as the railings of a Revit 2025 file are.

For each surface, and for each of several readings of `w` (the node's value times the cell's area,
minus that, none, unscaled, times the cell's width in u or in v, twice, half and a quarter of the area
reading; the tangents are as stage 2 reads them, the node's value times the cell's width), this
evaluates the bicubic patch of every cell and takes, for each vertex of Revit's body, the distance to
the closest point of the patch. The closest point is found, not sampled: a coarse grid of samples
(0.01 ft, at most 200 a side) names the nearest sample, and the cell's parameters are refined around it
three times, so that a vertex on the surface is found at 1e-5 ft where a sample alone leaves it 0.002 ft
off (the first version of this scorer sampled, and every reading looked alike).

A vertex is on a reading's surface when its closest point is within 0.006 ft. For each reading the
output says how many of the vertices on any reading's surface are within 0.0005 ft and within 0.001 ft
of its own, and the median and the 90th percentile of their distances. Revit's vertices that no reading
puts on the surface (those of other faces: caps, other solids) are not counted.

    python3 tools/re/hermite_twist_vs_ifc.py <probe.txt> <revit.ifc> [--list]

Measure passes `--list` after the two files, as to every probe scorer; this one has no list to give.
Needs IfcOpenShell and numpy.
"""

import collections
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mesh_bodies_vs_ifc as mesh  # noqa: E402

NEAR = 0.006  # feet: a vertex this close to the closest point of a patch is on it
BARS = (0.0005, 0.001)  # feet
SPACING = 0.01  # feet: the spacing of the coarse samples of a patch
MAX_SIDE = 200  # samples: the most a cell's side has, which bounds the cloud of a long cell
CANDIDATE_MIN = 0.012  # feet: a vertex this close to a coarse sample is refined
PASSES = 3


def read_surfaces(path):
    """((symbol, k) -> {u, v, nodes[(iu, iv)] -> (p, tu, tv, w)}, symbol -> [(Tag, rotation or None, origin or None)])."""
    out = {}
    users = collections.defaultdict(list)
    for line in open(path, encoding="utf-8", errors="replace"):
        parts = line.split()
        if not parts:
            continue
        if parts[0] == "instance" and len(parts) >= 4 and parts[2] == "symbol":
            rotation = origin = None
            if len(parts) == 4 + 1 + 9 + 1 + 3 and parts[4] == "rot" and parts[14] == "or":
                rotation = np.array([float(x) for x in parts[5:14]]).reshape(3, 3)
                origin = np.array([float(x) for x in parts[15:18]])
            users[parts[3]].append((parts[1], rotation, origin))
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


def place(surface, rotation, origin):
    """The surface with its nodes placed by an instance: points rotated and moved, tangents and twist rotated."""
    if rotation is None:
        return surface
    nodes = {
        key: (rotation @ p + origin, rotation @ tu, rotation @ tv, rotation @ w)
        for key, (p, tu, tv, w) in surface["nodes"].items()
    }
    return {"u": surface["u"], "v": surface["v"], "nodes": nodes}


def basis(t):
    return {
        "v0": 2 * t**3 - 3 * t**2 + 1,
        "v1": -2 * t**3 + 3 * t**2,
        "s0": t**3 - 2 * t**2 + t,
        "s1": t**3 - t**2,
    }


def cell_points(nodes, i, j, du, dv, twist, t, s):
    """The points of the cell (i, j)'s bicubic patch at parameters `t` (along u) and `s` (along v), in 0 to 1."""
    bt, bs = basis(t), basis(s)
    point = np.zeros(np.shape(t) + (3,))
    for b in (0, 1):
        for a in (0, 1):
            p, tu, tv, w = nodes[(i + a, j + b)]
            point += (bt[f"v{a}"] * bs[f"v{b}"])[..., None] * p
            point += (bt[f"s{a}"] * bs[f"v{b}"])[..., None] * (tu * du)
            point += (bt[f"v{a}"] * bs[f"s{b}"])[..., None] * (tv * dv)
            point += (bt[f"s{a}"] * bs[f"s{b}"])[..., None] * (w * twist(du, dv))
    return point


def coarse_cloud(surface, twist):
    """The samples of every cell's patch, and for each its cell, parameters, the cell's lengths and spacing."""
    u, v, nodes = surface["u"], surface["v"], surface["nodes"]
    points, meta = [], []
    for j in range(len(v) - 1):
        for i in range(len(u) - 1):
            du, dv = u[i + 1] - u[i], v[j + 1] - v[j]
            lu = max(np.linalg.norm(nodes[(i + 1, j)][0] - nodes[(i, j)][0]), np.linalg.norm(nodes[(i + 1, j + 1)][0] - nodes[(i, j + 1)][0]), 1e-6)
            lv = max(np.linalg.norm(nodes[(i, j + 1)][0] - nodes[(i, j)][0]), np.linalg.norm(nodes[(i + 1, j + 1)][0] - nodes[(i + 1, j)][0]), 1e-6)
            nt = int(min(max(np.ceil(lu / SPACING) + 1, 4), MAX_SIDE))
            ns = int(min(max(np.ceil(lv / SPACING) + 1, 4), MAX_SIDE))
            tt, ss = np.meshgrid(np.linspace(0.0, 1.0, nt), np.linspace(0.0, 1.0, ns), indexing="ij")
            points.append(cell_points(nodes, i, j, du, dv, twist, tt, ss).reshape(-1, 3))
            h = max(lu / (nt - 1), lv / (ns - 1))
            count = nt * ns
            meta.append(np.stack([np.full(count, i), np.full(count, j), tt.reshape(-1), ss.reshape(-1), np.full(count, lu), np.full(count, lv), np.full(count, h)], axis=1))
    return np.concatenate(points), np.concatenate(meta)


def nearest(cloud, queries, cell):
    """For each query the distance to its nearest point of `cloud` and that point's index, exact where within `cell`.

    The cloud is binned in cubes of `cell` feet, so any point within `cell` of a query is in the query's cube
    or one of its 26 neighbours; a query with none of them near gets infinity.
    """
    keys = np.floor(cloud / cell).astype(np.int64)
    order = np.lexsort((keys[:, 2], keys[:, 1], keys[:, 0]))
    sorted_cloud, keys = cloud[order], keys[order]
    starts = np.concatenate(([0], np.nonzero(np.any(keys[1:] != keys[:-1], axis=1))[0] + 1))
    ends = np.concatenate((starts[1:], [len(cloud)]))
    cubes = {tuple(int(c) for c in keys[s]): (s, e) for s, e in zip(starts, ends)}
    around = [(a, b, c) for a in (-1, 0, 1) for b in (-1, 0, 1) for c in (-1, 0, 1)]
    distance = np.full(len(queries), np.inf)
    index = np.zeros(len(queries), dtype=np.int64)
    for n, query in enumerate(queries):
        x, y, z = (int(c) for c in np.floor(query / cell))
        spans = [cubes[(x + a, y + b, z + c)] for a, b, c in around if (x + a, y + b, z + c) in cubes]
        if not spans:
            continue
        found = np.concatenate([np.arange(s, e) for s, e in spans])
        d = np.linalg.norm(sorted_cloud[found] - query, axis=1)
        m = int(d.argmin())
        distance[n], index[n] = d[m], order[found[m]]
    return distance, index


def closest(surface, twist, query, i, j, t0, s0, lu, lv, h):
    """The distance from `query` to the closest point of cell (i, j)'s patch, refined from the sample (t0, s0)."""
    u, v, nodes = surface["u"], surface["v"], surface["nodes"]
    du, dv = u[i + 1] - u[i], v[j + 1] - v[j]
    wt, ws = min(3 * h / lu, 0.5), min(3 * h / lv, 0.5)
    best = (np.inf, t0, s0)
    for _ in range(PASSES):
        centre_t, centre_s = (t0, s0) if best[0] == np.inf else (best[1], best[2])
        tt, ss = np.meshgrid(
            np.clip(np.linspace(centre_t - wt, centre_t + wt, 31), 0, 1),
            np.clip(np.linspace(centre_s - ws, centre_s + ws, 31), 0, 1),
            indexing="ij",
        )
        d = np.linalg.norm(cell_points(nodes, i, j, du, dv, twist, tt, ss) - query, axis=-1)
        m = np.unravel_index(int(d.argmin()), d.shape)
        best = (float(d[m]), float(tt[m]), float(ss[m]))
        wt, ws = wt / 6, ws / 6
    return best[0]


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


def distances_by_reading(surface, vertices):
    """reading -> for each vertex of Revit's body the distance to the closest point of the surface's patch (inf when far)."""
    out = {}
    for name, twist in CONVENTIONS.items():
        cloud, meta = coarse_cloud(surface, twist)
        cell = max(CANDIDATE_MIN, 1.5 * float(meta[:, 6].max()))
        coarse, index = nearest(cloud, vertices, cell)
        result = np.full(len(vertices), np.inf)
        for n in np.nonzero(np.isfinite(coarse))[0]:
            i, j, t0, s0, lu, lv, h = meta[index[n]]
            if coarse[n] <= max(CANDIDATE_MIN, 1.5 * h):
                result[n] = closest(surface, twist, vertices[n], int(i), int(j), t0, s0, lu, lv, h)
        out[name] = result
    return out


def twist_reach(surface):
    """The most the twist term moves a cell's patch, in feet, with `w` times the cell's area and unscaled."""
    u, v, nodes = surface["u"], surface["v"], surface["nodes"]
    k = (4 / 27) ** 2  # the largest |s_a(t) s_b(s)| of the two Hermite twist basis functions
    area = unscaled = 0.0
    for j in range(len(v) - 1):
        for i in range(len(u) - 1):
            norms = sum(np.linalg.norm(nodes[(i + a, j + b)][3]) for a in (0, 1) for b in (0, 1))
            area = max(area, norms * (u[i + 1] - u[i]) * (v[j + 1] - v[j]) * k)
            unscaled = max(unscaled, norms * k)
    return area, unscaled


def main():
    arguments = [argument for argument in sys.argv[1:] if argument != "--list"]
    if len(arguments) != 2:
        print(__doc__)
        return 2
    surfaces, users = read_surfaces(arguments[0])
    if not surfaces:
        print("hermite twist: no surface in the probe output")
        return 0
    # A surface is on its symbol's record; Revit's export has the elements that place the symbol.
    placed = {symbol: users.get(symbol) or [(symbol, None, None)] for symbol, _ in surfaces}
    revit = mesh.meshes(arguments[1], tags={tag for named in placed.values() for tag, _, _ in named})
    names = list(CONVENTIONS)
    print("hermite twist: readings, in the order of the counts below: " + "; ".join(f"{n + 1} {name}" for n, name in enumerate(names)))
    everything = {name: [] for name in names}
    for (symbol, k), surface in sorted(surfaces.items()):
        for tag, rotation, origin in placed[symbol]:
            if tag not in revit:
                print(f"hermite twist: symbol {symbol} surface {k}: no element of Revit's export with Tag {tag}")
                continue
            vertices = np.unique(np.round(revit[tag][2].reshape(-1, 3), 6), axis=0)
            here = place(surface, rotation, origin)
            distance = distances_by_reading(here, vertices)
            on = np.zeros(len(vertices), dtype=bool)
            for d in distance.values():
                on |= d <= NEAR
            if not on.any():
                print(f"hermite twist: symbol {symbol} (element {tag}) surface {k}: no vertex of Revit's body of {len(vertices)} on it")
                continue
            for name in names:
                everything[name].extend(distance[name][on].tolist())
            reach_area, reach_unscaled = twist_reach(here)
            counts = " ".join(str(int((distance[name][on] <= BARS[0]).sum())) for name in names)
            cells = (len(surface["u"]) - 1) * (len(surface["v"]) - 1)
            print(
                f"hermite twist: symbol {symbol} (element {tag}) surface {k}: {len(surface['u'])} by {len(surface['v'])} nodes, {cells} cells;"
                f" {int(on.sum())} vertices on it, within {BARS[0]} ft by reading: {counts};"
                f" the twist moves a cell at most {reach_area:.4f} ft (area), {reach_unscaled:.4f} ft (unscaled)"
            )
    total = len(everything[names[0]])
    print(f"hermite twist: all surfaces, {total} (vertex, surface) pairs with the vertex on the surface by some reading:")
    for name in names:
        d = np.array(everything[name])
        on_surface = d[d <= NEAR]
        print(
            f"hermite twist:   {name}: within {BARS[0]} ft: {int((d <= BARS[0]).sum())}; within {BARS[1]} ft: {int((d <= BARS[1]).sum())};"
            f" on the surface (within {NEAR} ft): {len(on_surface)};"
            f" median {float(np.median(on_surface)) if len(on_surface) else float('nan'):.6f} ft,"
            f" 90th percentile {float(np.percentile(on_surface, 90)) if len(on_surface) else float('nan'):.6f} ft"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())

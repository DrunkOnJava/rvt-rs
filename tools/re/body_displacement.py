#!/usr/bin/env python3
"""How far a change moves the bodies it moves (#2098).

    python3 tools/re/body_displacement.py <base.ifc> <head.ifc> [<max bodies>]

For every object drawn by triangulated face sets in both exports whose fingerprint differs
(`tools/re/ifc_body_diff.py`'s pairing by GlobalId), the farthest a vertex of the head's
mesh lies from the base's surface (its triangles), in the file's own coordinates (each
face set's points as written, which `ifc_body_diff` also compares), and the same the
other way. Bodies with the same pair of before-and-after fingerprints are measured once.
Prints one line per shape measured, how many bodies each shape covers, and the largest of
each distance over all of them.
"""
import os
import re
import sys

import numpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ifc_body_diff import ENTITY, refs, top_level_args, read  # noqa: E402

NUMBER = re.compile(r"[-+]?\d+(?:\.\d*)?(?:[eE][-+]?\d+)?")


def meshes(path, wanted):
    """GlobalId -> list of (points array, triangles array) for the objects in `wanted`."""
    entities = {}
    with open(path, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            m = ENTITY.match(line.rstrip("\n"))
            if m:
                entities[int(m.group(1))] = (m.group(2), m.group(3))
    kind = lambda i: entities.get(i, ("",))[0]
    out = {}
    for index, (k, body) in entities.items():
        if not k.startswith("IFC") or k.startswith("IFCREL"):
            continue
        args = top_level_args(body)
        if len(args) <= 7:
            continue
        gid = args[0].strip("'")
        if gid not in wanted:
            continue
        sets = []
        for shape in refs(args[6]):
            if kind(shape) != "IFCPRODUCTDEFINITIONSHAPE":
                continue
            for rep in refs(top_level_args(entities[shape][1])[2]):
                if kind(rep) != "IFCSHAPEREPRESENTATION":
                    continue
                for item in refs(top_level_args(entities[rep][1])[3]):
                    if kind(item) != "IFCTRIANGULATEDFACESET":
                        continue
                    fs = top_level_args(entities[item][1])
                    plist = top_level_args(entities[refs(fs[0])[0]][1])[0]
                    pts = numpy.array([float(x) for x in NUMBER.findall(plist)]).reshape(-1, 3)
                    idx = numpy.array([int(x) for x in NUMBER.findall(fs[3])]).reshape(-1, 3) - 1
                    sets.append((pts, idx))
        out[gid] = sets
    return out


def farthest(points, triangles):
    """The largest distance from any of `points` to the nearest of `triangles` (each 3x3)."""
    a, b, c = triangles[:, 0], triangles[:, 1], triangles[:, 2]
    n = numpy.cross(b - a, c - a)
    area = numpy.linalg.norm(n, axis=1)
    unit = n / numpy.where(area > 0, area, 1)[:, None]

    def seg(p, u, v):
        uv = v - u
        l2 = numpy.einsum("ij,ij->i", uv, uv)
        t = numpy.clip(numpy.einsum("ij,ij->i", p - u, uv) / numpy.where(l2 > 0, l2, 1), 0, 1)
        return numpy.linalg.norm(p - (u + t[:, None] * uv), axis=1)

    worst = 0.0
    for p in points:
        h = numpy.einsum("ij,ij->i", p - a, unit)
        foot = p - h[:, None] * unit
        inside = area > 0
        for u, v in ((a, b), (b, c), (c, a)):
            inside &= numpy.einsum("ij,ij->i", numpy.cross(v - u, foot - u), n) >= 0
        edge = numpy.minimum.reduce([seg(p, a, b), seg(p, b, c), seg(p, c, a)])
        worst = max(worst, float(numpy.min(numpy.where(inside, numpy.abs(h), edge))))
    return worst


def main():
    base_path, head_path = sys.argv[1], sys.argv[2]
    limit = int(sys.argv[3]) if len(sys.argv) > 3 else 10**9
    base, head = read(base_path), read(head_path)
    moved = sorted(g for g in base.keys() & head.keys() if base[g][4] != head[g][4] and base[g][2] == head[g][2])
    moved = moved[:limit]
    bm, hm = meshes(base_path, set(moved)), meshes(head_path, set(moved))
    largest = [0.0, 0.0]
    # Bodies drawn the same before and after (an instance of one family placed many times) move
    # alike, so each pair of fingerprints is measured once and its instances counted.
    shapes = {}
    for gid in moved:
        if not bm.get(gid) or not hm.get(gid):
            continue
        key = (base[gid][4], head[gid][4])
        if key in shapes:
            shapes[key][0] += 1
            continue
        tri = lambda sets: numpy.concatenate([p[i] for p, i in sets])
        bt, ht = tri(bm[gid]), tri(hm[gid])
        hp = numpy.concatenate([p for p, _ in hm[gid]])
        bp = numpy.concatenate([p for p, _ in bm[gid]])
        out_ = farthest(hp, bt)
        in_ = farthest(bp, ht)
        largest = [max(largest[0], out_), max(largest[1], in_)]
        shapes[key] = [1, gid]
        print(f"moved {gid} tag={head[gid][1]} class={head[gid][0]} head_to_base={out_:.6g} base_to_head={in_:.6g}", flush=True)
    for count, gid in shapes.values():
        print(f"shape of {gid} tag={head[gid][1]}: {count} bodies")
    print(f"bodies {len(moved)} shapes {len(shapes)} largest head_to_base={largest[0]:.6g} base_to_head={largest[1]:.6g}")


if __name__ == "__main__":
    main()

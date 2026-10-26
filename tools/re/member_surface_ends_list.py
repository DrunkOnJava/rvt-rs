#!/usr/bin/env python3
"""How many of a curtain mullion's end faces Revit's NetSurfaceArea counts
(#1016, W7).

Revit's export gives a curtain mullion drawn as an extrusion of its section a
`Qto_MemberBaseQuantities.NetSurfaceArea` that is its section's perimeter
times its `Length` plus 0, 1 or 2 times its section's area. For every IfcMember
both files write, it takes rvt-rs's body as a world-space box (IfcOpenShell's
geometry), its axis the box side equal to Revit's `Length`, and counts the
ends free of every other member's box (an end is covered where a point just
past its centre lies in another member's box, a 1 mm margin), and the ends
free of every box of a member running across it (its longest side on another
axis): a mullion meeting a collinear one end to end, as stacked curtain walls'
mullions do, is not joined to it. It tallies
Revit's count of ends against the free ends rvt-rs finds, and against the
world axis the member runs along and its type, and lists up to 30 where the
count and the free ends differ.

Usage:

    python3 tools/re/member_surface_ends_list.py <rvt-rs.ifc> <revit-export.ifc>
"""

import collections
import os
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.unit
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from qtos_vs_ifc import quantities  # noqa: E402

MARGIN = 0.001


def boxes(f):
    """Tag -> (min, max) world box of each IfcMember's body, metres."""
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    scale = ifcopenshell.util.unit.calculate_unit_scale(f)
    out = {}
    for member in f.by_type("IfcMember"):
        if not member.Tag or member.Representation is None:
            continue
        try:
            shape = ifcopenshell.geom.create_shape(settings, member)
        except Exception:
            continue
        verts = np.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
        if len(verts) == 0:
            continue
        # create_shape returns metres where the file's unit scale says so.
        out[f"tag:{member.Tag}"] = (verts.min(axis=0), verts.max(axis=0))
    return out


def main(argv):
    args = [a for a in argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    ours_f = ifcopenshell.open(args[0])
    mine = boxes(ours_f)
    revit = collections.defaultdict(dict)
    for (key, qset, name), (_, value) in quantities(args[1])[0].items():
        if qset == "Qto_MemberBaseQuantities" and value is not None:
            revit[key][name] = value
    keys = sorted(mine)
    lo = np.array([mine[k][0] for k in keys])
    hi = np.array([mine[k][1] for k in keys])
    # A coarse grid over box centres keeps the covering test near-linear.
    cell = 2.0
    grid = collections.defaultdict(list)
    for i, k in enumerate(keys):
        a = np.floor(lo[i] / cell).astype(int)
        b = np.floor(hi[i] / cell).astype(int)
        for x in range(a[0], b[0] + 1):
            for y in range(a[1], b[1] + 1):
                for z in range(a[2], b[2] + 1):
                    grid[(x, y, z)].append(i)

    long_axis = np.argmax(hi - lo, axis=1)

    def covered(point, me, across=False):
        c = tuple(np.floor(point / cell).astype(int))
        for j in grid.get(c, ()):
            if j == me or (across and long_axis[j] == long_axis[me]):
                continue
            if np.all(point >= lo[j] - MARGIN) and np.all(point <= hi[j] + MARGIN):
                return True
        return False

    tally = collections.Counter()
    across_tally = collections.Counter()
    by_axis = collections.Counter()
    by_type = collections.Counter()
    types = {f"tag:{m.Tag}": (getattr(m, "ObjectType", None) or "-").replace(" ", "_") for m in ifcopenshell.open(args[1]).by_type("IfcMember") if m.Tag}
    differ = []
    for i, k in enumerate(keys):
        r = revit.get(k, {})
        nsa, length = r.get("NetSurfaceArea"), r.get("Length")
        if nsa is None or length is None:
            continue
        size = hi[i] - lo[i]
        axis = int(np.argmin(np.abs(size - length)))
        if abs(size[axis] - length) > max(1e-4, 1e-5 * length):
            tally[("axis not found", "-")] += 1
            continue
        across = [size[a] for a in range(3) if a != axis]
        section, perimeter = across[0] * across[1], 2 * (across[0] + across[1])
        if section <= 0:
            continue
        ends = (nsa - perimeter * length) / section
        revit_ends = round(ends)
        if abs(ends - revit_ends) > 1e-3:
            tally[("not a whole number of ends", "-")] += 1
            continue
        centre = (lo[i] + hi[i]) / 2
        free = free_across = 0
        for sign in (-1, 1):
            point = centre.copy()
            point[axis] = (hi[i][axis] if sign > 0 else lo[i][axis]) + sign * 2 * MARGIN
            free += not covered(point, i)
            free_across += not covered(point, i, across=True)
        tally[(revit_ends, free)] += 1
        across_tally[("z" if axis == 2 else "xy", revit_ends, free_across)] += 1
        by_axis[("xyz"[axis], revit_ends)] += 1
        by_type[(types.get(k, "-"), "z" if axis == 2 else "xy", revit_ends)] += 1
        if revit_ends != free:
            differ.append((k, revit_ends, free, axis, size))
    print("Revit's ends counted, free ends rvt-rs finds: members")
    for (a, b), n in sorted(tally.items(), key=lambda kv: -kv[1]):
        print(f"  {a} {b}: {n}")
    print("vertical (z) or not, Revit's ends counted, ends free of every member across it: members")
    for (a, b, c), n in sorted(across_tally.items()):
        print(f"  {a} {b} {c}: {n}")
    print("world axis, Revit's ends counted: members")
    for (a, b), n in sorted(by_axis.items()):
        print(f"  {a} {b}: {n}")
    print("Revit's type, vertical (z) or not, Revit's ends counted: members")
    for (t, a, b), n in sorted(by_type.items(), key=lambda kv: -kv[1])[:30]:
        print(f"  {t} {a} {b}: {n}")
    print("differing:")
    for k, a, b, axis, size in differ[:30]:
        print(f"  {k} Revit {a} free {b} axis {'xyz'[axis]} size {np.round(size, 4).tolist()}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

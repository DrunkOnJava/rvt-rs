"""#2085 row 4: each body newly drawn from a saved mesh, against Revit's export, on every vertex.

Reads the Tags `ifc_body_diff.py` wrote for the objects drawn from a saved mesh in the head and not in
the base, meshes each in the head export and in Revit's with `mesh_bodies_vs_ifc.meshes`, and gives
the farthest of every distinct vertex from the other body's surface both ways
(`mesh_bodies_vs_ifc.surface_distance`) and the volume ratio (`mesh_bodies_vs_ifc.volume`).

Usage: python3 newbodies2085.py TOOLS_RE_DIR HEAD.ifc REVIT.ifc NEWLY_DRAWN.txt
"""

import sys

import numpy as np


def distinct(triangles):
    points = triangles.reshape(-1, 3)
    points = points[np.lexsort((points[:, 2], points[:, 1], points[:, 0]))]
    return points[np.r_[True, (points[1:] != points[:-1]).any(axis=1)]]


def main():
    tools, head, revit, listing = sys.argv[1:5]
    sys.path.insert(0, tools)
    import mesh_bodies_vs_ifc as M

    tags = set()
    for line in open(listing):
        parts = line.split()
        if len(parts) >= 2:
            tags.add(parts[1])
    print(f"newly drawn: {len(tags)}")
    if not tags:
        return
    ours = M.meshes(head, tags=tags, source=M.SOURCE)
    ref = M.meshes(revit, tags=set(ours))
    within = 0
    for tag in sorted(tags, key=lambda t: (len(t), t)):
        if tag not in ours:
            print(f"  {tag}: not meshed in the head")
            continue
        if tag not in ref:
            print(f"  {tag} {ours[tag][0]}: not in Revit's export")
            continue
        cls, object_type, o = ours[tag]
        r = ref[tag][2]
        ratio = M.volume(o) / M.volume(r) if M.volume(r) > 0 else float("inf")
        to_ours = M.surface_distance(distinct(r), o)
        to_ref = M.surface_distance(distinct(o), r)
        ok = max(to_ours, to_ref) <= 0.01 and abs(ratio - 1) <= 0.01
        within += ok
        print(
            f"  {tag} {cls} '{object_type}': Revit's vertices to rvt-rs's surface {to_ours:.6f} ft,"
            f" rvt-rs's to Revit's {to_ref:.6f} ft, volume ratio {ratio:.4f}" + ("" if ok else "  MISSES")
        )
    print(f"within 0.01 ft both ways and 1% volume: {within}")


if __name__ == "__main__":
    main()

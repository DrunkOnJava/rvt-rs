"""Which shell of a body each listed point belongs to.

    python whichshell.py <ifc> <Tag> <x,y,z> ...

Points are in the frame far_vertices_vs_ifc prints (the file's site undone, in feet). The body
is meshed in world coordinates, moved into that frame with ifc_common.site_frame, split into its
connected shells, and each point is matched to its nearest vertex. Prints, per point, the
distance to that vertex and the shell's triangle count and extents along its principal axes (ft).
"""
import os
import sys

import ifcopenshell
import ifcopenshell.geom
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "tools", "re"))
from ifc_common import site_frame  # noqa: E402
from legs import shells  # noqa: E402


def main():
    path, tag = sys.argv[1], sys.argv[2]
    wanted = [np.array([float(x) for x in a.split(",")]) for a in sys.argv[3:]]
    model = ifcopenshell.open(path)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    element = next(e for e in model.by_type("IfcProduct") if str(getattr(e, "Tag", "")) == tag and e.Representation)
    shape = ifcopenshell.geom.create_shape(settings, element)
    verts = site_frame(model)(np.array(shape.geometry.verts, dtype=float).reshape(-1, 3))
    faces = np.array(shape.geometry.faces, dtype=int).reshape(-1, 3)
    owner = np.full(len(verts), -1)
    described = []
    for k, tris in enumerate(sorted(shells(verts, faces), key=len, reverse=True)):
        owner[faces[tris].ravel()] = k
        pts = verts[faces[tris]].reshape(-1, 3)
        centred = pts - pts.mean(axis=0)
        _, _, axes = np.linalg.svd(centred, full_matrices=False)
        described.append((len(tris), [round(float(np.ptp(centred @ ax)), 4) for ax in axes]))
    for k, (n, ext) in enumerate(described):
        print(f"shell {k}: triangles={n} extents_ft={ext}")
    for w in wanted:
        d = np.linalg.norm(verts - w, axis=1)
        i = int(np.argmin(d))
        print(f"point {tuple(w)}: nearest vertex {d[i]:.5f} ft away, shell {owner[i]}")


main()

#!/usr/bin/env python3
"""Check the I-section beams rvt-rs writes, as IfcOpenShell meshes them.

Research tool for `reports/element-framing/RE-103-beam-i-sections.md` (#94).
For every IfcBeam whose `SectionShape` property is `I`, it meshes the beam
with IfcOpenShell and checks:

- volume: the mesh's volume equals the IfcIShapeProfileDef's area (two
  flanges and the web between them) times the beam's `AxisLength`;
- orientation: across the extrusion axis, level, the mesh is the flange
  width wide, and square to that it is the section's depth.

Usage:

    python3 tools/re/beam_sections_in_ifc.py <rvt-rs.ifc>

Needs IfcOpenShell (tested with 0.8.5) and NumPy.
"""

import sys, ifcopenshell, ifcopenshell.geom, ifcopenshell.util.element as ue, numpy as np, collections
f = ifcopenshell.open(sys.argv[1])
s = ifcopenshell.geom.settings(); s.set("use-world-coords", True)
c = collections.Counter(); worst = 0.0
for b in f.by_type("IfcBeam"):
    ps = {k: v for p in ue.get_psets(b).values() for k, v in p.items()}
    if ps.get("SectionShape") != "I": continue
    shape = ifcopenshell.geom.create_shape(s, b)
    v = np.array(shape.geometry.verts).reshape(-1, 3)
    t = np.array(shape.geometry.faces).reshape(-1, 3)
    prof = next(it for it in f.traverse(b.Representation) if it.is_a("IfcIShapeProfileDef"))
    W, D, tw, tf = prof.OverallWidth, prof.OverallDepth, prof.WebThickness, prof.FlangeThickness
    a, bb, cc = v[t[:, 0]], v[t[:, 1]], v[t[:, 2]]
    vol = abs(np.einsum("ij,ij->i", a, np.cross(bb, cc)).sum() / 6.0)
    area = 2 * W * tf + (D - 2 * tf) * tw
    rel = abs(vol / area - ps["AxisLength"]) / ps["AxisLength"]
    worst = max(worst, rel)
    c["volume = I area x axis length within 0.1%" if rel < 1e-3 else "volume differs"] += 1
    ext = next(it for it in f.traverse(b.Representation) if it.is_a("IfcExtrudedAreaSolid"))
    ax = np.array(ext.Position.Axis.DirectionRatios); ax /= np.linalg.norm(ax)
    plan = np.hypot(ax[0], ax[1]); across = np.array([-ax[1] / plan, ax[0] / plan, 0.0]); up = np.cross(ax, across)
    ok = abs(np.ptp(v @ across) - W) < 1e-4 and abs(np.ptp(v @ up) - D) < 1e-4
    c["flange width across, depth upright" if ok else "turned"] += 1
print(dict(c), "worst %.2e" % worst)

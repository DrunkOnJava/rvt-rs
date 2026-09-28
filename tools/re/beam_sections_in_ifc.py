#!/usr/bin/env python3
"""Check the I-section beams and columns rvt-rs writes, as IfcOpenShell
meshes them.

Research tool for `reports/element-framing/RE-103-beam-i-sections.md` and
`RE-104-column-i-sections.md` (#94). For every IfcBeam and IfcColumn whose
`SectionShape` property is `I`, it meshes the element with IfcOpenShell and
checks:

- volume: the mesh's volume equals the IfcIShapeProfileDef's area (two
  flanges and the web between them) times the beam's `AxisLength`, or the
  column's height;
- orientation: a beam is the flange width wide level across its extrusion
  axis and the section's depth square to that; a column has, in its own
  placement's axes, a vertex at each outer corner and inner flange tip of
  the section;
- material (RE-105): the element's material is an IfcMaterialProfileSetUsage
  whose one profile is an IfcIShapeProfileDef equal to the body's.

Usage:

    python3 tools/re/beam_sections_in_ifc.py <rvt-rs.ifc>

Needs IfcOpenShell (tested with 0.8.5) and NumPy.
"""

import sys, ifcopenshell, ifcopenshell.geom, ifcopenshell.util.element as ue, numpy as np, collections
f = ifcopenshell.open(sys.argv[1])
s = ifcopenshell.geom.settings(); s.set("use-world-coords", True)
c = collections.Counter(); worst = 0.0
import ifcopenshell.util.placement

for b in f.by_type("IfcBeam") + f.by_type("IfcColumn"):
    ps = {k: v for p in ue.get_psets(b).values() for k, v in p.items()}
    if ps.get("SectionShape") != "I": continue
    shape = ifcopenshell.geom.create_shape(s, b)
    v = np.array(shape.geometry.verts).reshape(-1, 3)
    t = np.array(shape.geometry.faces).reshape(-1, 3)
    prof = next(it for it in f.traverse(b.Representation) if it.is_a("IfcIShapeProfileDef"))
    usage = ue.get_material(b, should_skip_usage=False)
    held = (
        usage.ForProfileSet.MaterialProfiles[0].Profile
        if usage is not None and usage.is_a("IfcMaterialProfileSetUsage")
        else None
    )
    same = held is not None and held.is_a("IfcIShapeProfileDef") and all(
        abs(getattr(held, a) - getattr(prof, a)) < 1e-9
        for a in ("OverallWidth", "OverallDepth", "WebThickness", "FlangeThickness")
    )
    c["material profile = body's I" if same else "no material profile of its I"] += 1
    W, D, tw, tf = prof.OverallWidth, prof.OverallDepth, prof.WebThickness, prof.FlangeThickness
    a, bb, cc = v[t[:, 0]], v[t[:, 1]], v[t[:, 2]]
    vol = abs(np.einsum("ij,ij->i", a, np.cross(bb, cc)).sum() / 6.0)
    area = 2 * W * tf + (D - 2 * tf) * tw
    if b.is_a("IfcColumn"):
        m = ifcopenshell.util.placement.get_local_placement(b.ObjectPlacement)
        ux, uy, o = m[:3, 0], m[:3, 1], m[:3, 3]
        local = np.stack([(v - o) @ ux, (v - o) @ uy], axis=1)
        tip = D / 2 - tf
        want = [(sx * W / 2, sy * y) for sx in (-1, 1) for sy in (-1, 1) for y in (D / 2, tip)]
        found = all(np.min(np.hypot(local[:, 0] - a, local[:, 1] - c)) < 1e-4 for a, c in want)
        c["column: corners and flange tips in its own axes" if found else "column: turned"] += 1
        length = np.ptp(v[:, 2])
        rel = abs(vol / area - length) / length
        c["column: volume = I area x height within 0.1%" if rel < 1e-3 else "column: volume differs"] += 1
        continue
    rel = abs(vol / area - ps["AxisLength"]) / ps["AxisLength"]
    worst = max(worst, rel)
    c["volume = I area x axis length within 0.1%" if rel < 1e-3 else "volume differs"] += 1
    ext = next(it for it in f.traverse(b.Representation) if it.is_a("IfcExtrudedAreaSolid"))
    ax = np.array(ext.Position.Axis.DirectionRatios); ax /= np.linalg.norm(ax)
    plan = np.hypot(ax[0], ax[1]); across = np.array([-ax[1] / plan, ax[0] / plan, 0.0]); up = np.cross(ax, across)
    ok = abs(np.ptp(v @ across) - W) < 1e-4 and abs(np.ptp(v @ up) - D) < 1e-4
    c["flange width across, depth upright" if ok else "turned"] += 1
print(dict(c), "worst %.2e" % worst)

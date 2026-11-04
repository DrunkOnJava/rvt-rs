- **The body scorers measure shapes beside IfcOpenShell's default tessellation
  (#1355, RE-331).** `mesh_bodies_vs_ifc`, `saved_meshes_vs_ifc` and
  `family_extrusion_bodies_vs_ifc` mesh both IFC files with IfcOpenShell's
  default settings, so a circle Revit 24.0.5 writes as two half arcs and one
  Revit 26.2 writes as an `IfcCircleProfileDef` come out as 28 and 27 chords at
  69.85 mm, 0.0014 ft apart, although both are one exact circle. With Measure's
  new `fine_mesh` input (`RVT_SCORER_FINE_MESH=1`) the scorers also mesh both
  files at one equal, fine deflection (1e-5 ft, 0.03 rad) and print labelled
  `fine` rows after their own: the same columns, which bodies hold "ref on
  ours" in the default tessellation, the fine one, both or neither, and the
  bodies that miss in the fine one. Without it their output is as it was
  (compared on 20 pairs of files), and no figure or bar changes. In the fine
  tessellation the two forms of a circle from 0.02 to 2 ft in radius coincide
  within 1e-5 ft at every start, and RE1 Architecture's toilet-paper
  dispenser, 0.001439 ft off in the default tessellation, is 0.000002 ft off.
  Report: `reports/element-framing/RE-331-scorer-tessellation.md`.

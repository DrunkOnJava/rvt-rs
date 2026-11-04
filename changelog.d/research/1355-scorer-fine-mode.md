- **The body scorers measure shapes beside IfcOpenShell's default tessellation
  (#1355, RE-331).** `mesh_bodies_vs_ifc`, `saved_meshes_vs_ifc` and
  `family_extrusion_bodies_vs_ifc` mesh both IFC files with IfcOpenShell's
  default settings, so a circle Revit 24.0.5 writes as two half arcs and one
  Revit 26.2 writes as an `IfcCircleProfileDef` come out as 28 and 27 chords at
  69.85 mm, 0.0014 ft apart, although both are one exact circle. With Measure's
  new `fine_mesh` input (`RVT_SCORER_FINE_MESH=1`) the scorers also mesh both
  files at one equal, fine deflection (1e-5 ft, 0.03 rad) and print labelled
  `fine` rows after their own: the same columns, how many bodies are meshed
  differently, which hold "ref on ours" in the default tessellation, the fine
  one, both or neither, and the bodies that miss in the fine one. Without it
  their output is as it was: the default rows of the two flag runs equal the
  earlier runs of `main`'s scorers, 99 of 99 files in each (41850764234 and
  41850769925 against 41843267888 and 41843276506). In the fine tessellation
  the two forms of a circle from 0.02 to 2 ft in radius coincide within 1e-5
  ft at every start, and RE1 Architecture's toilet-paper dispenser, redrawn the
  way rvt-rs writes it, is 0.000002 ft off Revit's body where the default
  tessellation has it 0.001439 ft off. Measured on the models: none of the
  1,368 doors and windows the default export draws from saved meshes, and none
  of the 66 family-extrusion bodies, is meshed differently or changes verdict;
  of 3,816 saved-mesh bodies 295 are curved, 38 (the office's waste-terminal
  meshes, over the bar by at most 1.1e-6 ft) hold "ref on ours" only in the
  default tessellation and 158 hold both directions within 0.001 ft only in
  the fine one. No figure and no bar changes here; the options for the bar are
  in the report, and the choice is the maintainer's. Report:
  `reports/element-framing/RE-331-scorer-tessellation.md`.

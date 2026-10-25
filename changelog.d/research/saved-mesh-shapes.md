- **Saved display meshes scored by shape (RE-78, W6).** A new scorer,
  `tools/re/mesh_bodies_vs_ifc.py`, compares each body `--saved-meshes`
  draws with Revit's body for the same element, by volume and by how far
  each body's vertices lie from the other's surface. Every vertex is within
  0.001 ft on all 138 of Core Interior's doors and windows, on 1,294 of 1,709
  bodies scored on the BIM4LCA office (all 269 windows) and on 148 of 214 on
  RE1's Revit 2025 models. A mesh that is not exact is part of the body, and
  nothing in the file yet tells it apart, so the option stays opt-in.

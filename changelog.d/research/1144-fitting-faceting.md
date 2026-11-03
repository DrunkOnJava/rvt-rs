- **Why saved pipe fittings measure 1.2 times Revit's volume (RE-311).**
  Revit's export of a pipe elbow is a polyhedron of 6, 8 or 10 sides round
  the pipe and two segments along its bend, inscribed in the surface: 16 to
  25% short of the true torus, which rvt-rs's mesh reaches to within 1 to 2%.
  The closet bends' wrong volumes (0.28 to 2.59, one negative) were the
  scorer's, which took a mesh with cracks between its faces about the file's
  origin; `mesh_bodies_vs_ifc.py` now takes it about the body's centre (the
  three RE1 Mechanical bends: 1.1431 each) and counts the bodies whose Revit
  vertices all lie on the mesh. `fitting_faces_vs_ifc.py` and
  `probe_re311_fitting_faces` set each face of a saved fitting against
  Revit's body (#1144).

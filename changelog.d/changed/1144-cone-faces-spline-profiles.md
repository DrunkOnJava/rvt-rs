- **Saved meshes read cone faces and spline profile curves.** A `ConeSurf`
  face failed with `unsupported surface ConeSurf`, and a `GNurbSpline` or
  `GHermiteSpline` profile of a ruled surface or a surface of revolution with
  `unsupported parametric profile curve`, which kept the whole element's box
  (RE-224). The cone is read as a cone of revolution about its apex, `v` the
  distance along the generator; the splines are evaluated, their grid cut by
  chord error. With `--saved-meshes` RE1 Plumbing draws 8 more pipe
  transitions (every Revit vertex within 0.001 ft of the mesh, all within
  0.01 ft of Revit's body, 1.08 to 1.10 of its volume: Revit's export is a
  6- or 8-sided polyhedron). No other RE1 model changes, and the two RE1
  Electrical panels #1144 names keep their box for trims RE-302 and RE-305
  measured (#1144, RE-311).

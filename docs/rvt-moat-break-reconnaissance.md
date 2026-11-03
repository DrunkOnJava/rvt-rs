# Moved
The reconnaissance report is now [format-reconnaissance.md](format-reconnaissance.md).

## Addendum — RE-311 pipe fittings, cone faces and spline profiles (2026-11-03)

A saved pipe fitting is 1.08 to 1.32 of Revit's volume because Revit's export of it is a polyhedron of 6 to 10 sides round the pipe and two segments along a 90° bend, inscribed in the surface the saved graphics describe: an elbow's true torus is `π r² R π/2`, rvt-rs's mesh is 98 to 99% of it and Revit's 75 to 85%. The "wrong" volumes of the closet bends were the scorer's, which took them about the file's origin though a mesh with cracks between faces changes with its distance from it. A `ConeSurf` is `apex + v (cos α · z + sin α (cos u · x + sin u · y))` with `v` along the generator (the radius at `v` is `v sin α`), which draws 8 more RE1 Plumbing transitions. `GNurbSpline` and `GHermiteSpline` profile curves are read; the two RE1 Electrical panels still keep their box for trims that RE-302 and RE-305 measured. Report: `reports/element-framing/RE-311-fitting-faceting-cones-splines.md`.

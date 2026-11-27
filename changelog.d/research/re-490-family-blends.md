- **A blend of two straight-edged outlines is rebuilt from its records
  (RE-490).** `FormDocument::blend` reads a `BlendElem`: its two sketches
  (`m_sketchIds`), their planes and the two offsets (`Double` parameters
  -1001804 and -1001805), and gives each section's vertices in the family's
  coordinates, the solid's exact volume (each corner runs straight to its
  counterpart, so the section area is quadratic along the blend and
  Simpson's rule is exact) and its bounding box. On the five blends of two
  Revit 2020 families (one column family, four in a DynamoRevit test family) the
  volume agrees with the saved mesh to 3e-15 of it, the bounding box
  exactly, and every corner lies on a saved mesh vertex, with the first
  sketch at the first offset; the other pairing misses by 0.12 to 0.47 ft.
  Not read: arcs in a section, holes, sections of different vertex counts
  or not in parallel planes, a swept blend, and the corner-to-corner
  correspondence where the loops start at different vertices (measured only
  where the loops' order matches). Contributed by mageaustralia.

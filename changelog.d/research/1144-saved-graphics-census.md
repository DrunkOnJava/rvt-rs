- **What keeps a saved mesh from decoding whole, face by face (RE-329,
  #1144, #1247).** `probe_re329_saved_graphics_census` meshes every face of
  an element that keeps its box, not only the first that fails, and prints
  counts: each failing face's surface, profile curve and trim, and each
  element's set of causes, split into doors and windows (drawn by default)
  and everything else (`--saved-meshes`). On the BIM4LCA office as built 146
  of 11,246 candidates keep their box (147 of 11,363 at permit): 43 toilets
  that need ellipse profiles, curved ruled surfaces and trimmed
  revolutions together (RE-302), 30 fittings held by cylinder faces with
  several loops alone (RE-305), 19 sanitary terminals that need
  `HermiteSurf` and `GHermiteSpline` together, and 25 railings and terminals
  held by the graphics walk alone, not by a face; no door or window of the
  office, Core Interior or RE1 is among them (Measure run 41786625424).
  RE-305's 30 fittings, read on today's `main` with the volume taken about
  each body's centre, are still 1.1102 to 1.1105 of Revit's (run 41787545512)
  and, face by face, draw no surface Revit lacks (run 41788644583: 2.3% more
  area, no face off Revit's surface). They fail "Revit's vertices within
  0.001 ft" on 72 of Revit's 1,083 vertices, all nearest surfaces of
  revolution, up to 0.0051 ft: 26 in the pattern of RE-302's open item, a
  junction between two curved faces, and 46 at 0.0011 to 0.0023 ft on three
  other faces, for which no cause is shown. The reader stays off `main`. Not
  claimed: that the excess volume is wholly Revit's faceting of thin rods,
  or that any of the 72 vertices is Revit's error and not the mesh's.

- **Two round faces of a saved mesh meet where they share an edge (#2104,
  RE-495).** Where two cylinder faces share an edge that runs across both
  their parameters, as on pipe tees, fittings and fixtures, each put its own
  points between the edge's saved ones and their boundaries parted, by up to
  0.0016 ft and on 3,170 vertices over 0.001 ft across the open models.
  The edge now takes the points where the two cylinders meet, solved onto both
  and held by both faces, and none is left over 0.001 ft on the 77 files read.
  With #2111's half-chord bound (RE-496), the cost against the release before,
  on the files read: the bodies with such faces hold up to 1.13 times the
  triangles (Snowdon Towers HVAC; exports at most 2.0 MiB larger, on Autodesk's
  `rme_basic`), and on Snowdon Towers Architectural more of Revit's vertices lie
  beyond 0.001 ft of the bodies that move (23,243 to 23,427); whether that is a
  worse fit is not shown (#2111).

- **Two round faces of a saved mesh meet where they share an edge (#2104,
  RE-495).** Where two cylinder faces share an edge that runs across both
  their parameters, as on pipe tees, fittings and fixtures, each put its own
  points between the edge's saved ones and their boundaries parted, by up to
  0.0016 ft and on 3,170 vertices over 0.001 ft across the open models.
  The edge now takes the points where the two cylinders meet, solved onto both
  and held by both faces, and none is left over 0.001 ft on the 77 files read.
  The cost: on the plumbing and HVAC samples the bodies with such faces hold
  up to 1.35 times the triangles (exports at most 6.6 MiB larger), and on Snowdon Towers
  Architectural 45 of the 62 bodies that move fit Revit's own export slightly
  worse, as does one toilet on RE1 Architecture; why is not yet read (#2111).

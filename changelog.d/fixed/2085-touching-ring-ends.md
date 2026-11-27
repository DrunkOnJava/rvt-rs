- **A flat or round face of a saved mesh whose stored edges stop short of each
  other by less than 0.00001 ft draws (#2085, RE-497).** Revit saves some faces' edges
  with their ends a hair apart; rvt-rs refused each such face as a disconnected
  boundary, and with it the body. Ends closer than 0.00001 ft along the surface
  now count as one point, and no edge is added. On the BIM4LCA offices two
  curtain mullions now draw from their saved meshes, matching Revit's export to
  0.000003 ft, and four walls on the BIM4LCA files gain their base quantities,
  each equal to Revit's. Wider gaps stay refused.

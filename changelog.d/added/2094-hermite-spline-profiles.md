- **A ruled surface or a surface of revolution whose profile is a Hermite
  spline is read and drawn from the file's saved graphics (#2094, #2082,
  RE-492).** Such a face failed with `unsupported parametric profile curve
  GHermiteSpline`, and its body kept its record box. On the BIM4LCA office
  the timber version's two stair flights are now drawn in both saves. Every
  vertex of Revit's own body is within 0.000109 ft of rvt-rs's surface, and
  the volume is 112.116 ft³ against Revit's 112.115. On the terrain files a
  ramp and its ramp flight are drawn within 0.00088 ft of Revit's. Where a
  plane meets such a face along an edge that crosses both of its parameters,
  both faces now draw the edge straight between the points the file saves
  for it, as Revit's export does on the flights, so the two faces meet. In
  all, 25 bodies on 10 files are newly drawn. The 17 on Autodesk's
  `rac_advanced` saves and on Snowdon Towers Architectural have no Revit
  export to score them. Bodies that hold a Hermite profile and something
  else rvt-rs does not read stay boxes, now refused for that other part:
  the BIM4LCA toilets, RE1 Electrical's two panels and `rac_basic`'s one.

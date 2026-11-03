- **A roof sloped by several slope arrows is drawn as the planes they
  define, where Revit's export is measured to resolve them so.** Each
  arrow's plane rises from the boundary line its tail is on, at the arrow's
  slope, with the top a plane's own thickness over the cosine of its angle
  above the underside, and the planes meet in hips and valleys. On
  puzzbobb's `20_T_U_Roof` (Revit 2025) the T-shaped roof 4149 is now
  Revit's body, its farthest vertex 3.2e-06 ft from Revit's and its volume
  436.7078 ft³ against Revit's 436.7074 (it was the outline over the record
  box, 4,426.4 ft³). It is drawn on Revit 2025 only, for arrows set by
  their heights. The U-shaped roof 5941 stays its record box: Revit slopes
  its two long outer walls along the arms only, and a roof of its
  configuration (a sloped edge met inside by a reflex edge's extension,
  beside a sloped neighbour at a convex corner) is not drawn either, though
  only the U is measured (#885, RE-307).

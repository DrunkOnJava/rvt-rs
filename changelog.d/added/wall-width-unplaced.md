- **Every wall's `Width` (#1016).** A wall the partition reader does not draw
  from its centreline (a slanted wall, or one whose line its data does not
  place) carries `Qto_WallBaseQuantities.Width` too, its type's layers'
  thickness, which Revit's export writes as the wall's `Width` whatever its
  orientation: now all 607 of the BIM4LCA office's walls as built and all
  589 at permit, each Revit's value, and 31 more door and window openings'
  `Depth` with them.

- **The opening a door cuts in a second wall carries
  `Qto_OpeningElementBaseQuantities.Depth` and `Height` (#1165, RE-338).**
  `Depth` is the extent the opening is extruded across that wall by, the door's
  own width, and `Height` its outline's height; Revit's export measures this
  opening the same way round, not swapped as on a door's own opening. The
  opening's `Width`, which Revit's export writes too, stays out: Revit's is the
  thickness of the wall the door stands in, and rvt-rs's outline is 0.2 mm wider
  than that on two of the four openings read.

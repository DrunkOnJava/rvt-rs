- **An opening a floor's sketch makes carries the `Qto_OpeningElementBaseQuantities.Area` Revit's export gives it where its outline is not a rectangle, that outline's area (#1016).**
  The notched openings of the BIM4LCA office files gain Revit's `Area` within
  1e-6 m2 and the rectangular ones gain none, on which Revit's export writes
  none either. An outline with a corner that is not a right angle gains none:
  an arc of the sketch reaches it as chords, whose polygon is short of the
  area Revit takes from the curves. Nothing else of an opening changes, and an
  export made with `--no-base-quantities` gains none.

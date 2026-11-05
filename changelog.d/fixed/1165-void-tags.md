- **A floor's void is tagged with the sketch line Revit's export tags it with
  (#1165, RE-339).** RE-151 tagged each void with the lowest ElementId of the
  sketch lines on its loop, which is Revit's on 27 of the 36 voids of the
  BIM4LCA office as built and 27 of the 35 at building permit. On the others
  Revit's tag is the highest ElementId of the loop's lines: the lines taken in
  ElementId order, which is the order they were drawn in, run clockwise, and
  Revit's export starts a clockwise loop at the line drawn last. The tag is now
  the lowest where they run counter-clockwise and the highest where they run
  clockwise, and a void whose lines do not step round the loop the same way is
  left out instead of tagged by a guess. Measured figures are filled from the
  Measure runs of this change.

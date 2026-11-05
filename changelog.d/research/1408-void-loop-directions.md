- **The direction a floor void's first line is stored in says which line Revit
  tags the void with, on 133 of 133 voids (RE-355, #1408).** Of the sketch lines
  of a void's loop, the one with the lowest ElementId runs along the loop's
  counter-clockwise walk or against it; against it Revit tags that line, along it
  the line after it, which gives Revit's tag on the BIM4LCA office as built (36
  of 36) and at building permit (35 of 35) and on Core Interior (62 of 62),
  including the void 3147725 that RE-339 left wrong. A reading from the order the
  first two lines were drawn in is right on the office and wrong on 20 of Core
  Interior's 62, so it does not ship. `probe_re355_void_loop_directions` prints
  both, line by line, from Measure runs 42115473363, 42116875940 and 42118790534.
  The export is unchanged: `void_tag` does not read a line's stored direction
  yet, so 3147725 still takes the lowest id, 1 of 36 and 1 of 35 floor voids of
  the office files, and the held-out models are not read.

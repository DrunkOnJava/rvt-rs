- **A floor's void is tagged with the sketch line Revit's export tags it with
  where the lines run clockwise (#1165, RE-339).** RE-151 tagged each void
  with the lowest ElementId of the sketch lines on its loop, which is Revit's
  on 27 of the 36 voids of the BIM4LCA office as built and 27 of the 35 at
  building permit, and on all 62 floor and shading-device voids of Core
  Interior. On 8 and 7 of the others Revit's tag is the highest ElementId of
  the loop's lines: the lines taken in ElementId order, which is the order
  they were drawn in, run clockwise round the loop, and Revit's export starts
  a clockwise loop at the line drawn last. The tag is now the highest where
  the lines step clockwise all the way round the loop in that order, and the
  lowest everywhere else. On the office the voids with Revit's tag go from 27
  to 35 of 36 as built and from 27 to 34 of 35 at permit, with the box, name,
  type and `Reference` equal on every one, and the voids written under
  another line's Tag go from 9 and 8 to 1 on each file, a loop of 6 lines
  drawn out of order. Core Interior's openings and every other scorer output
  are unchanged (Measure run 41947422624).

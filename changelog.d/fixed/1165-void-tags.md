- **A floor's void is tagged with the sketch line Revit's export tags it with
  where the lines run clockwise (#1165, RE-339).** RE-151 tagged each void
  with the lowest ElementId of the sketch lines on its loop, which is Revit's
  on 27 of the 36 voids of the BIM4LCA office as built and 27 of the 35 at
  building permit, and on all 62 floor and shading-device voids of Core
  Interior. On 8 and 7 of the others Revit's tag is the highest ElementId of
  the loop's lines: taken in ElementId order, which is the order they were
  drawn in, they run clockwise round the loop, and Revit starts such a loop at
  the line drawn last. That is what 8 voids of one exporter show; why is not
  read. The tag is now the highest where at least three straight lines, each
  on an edge of its own, step exactly one edge clockwise all the way round the
  loop in that order (the closing step included), and the lowest everywhere
  else, a loop with a curve included. On the office the voids with Revit's tag
  go from 27 to 35 of 36 as built and from 27 to 34 of 35 at permit, with the
  box, name, type and `Reference` equal on every one, and the voids written
  under another line's Tag go from 9 and 8 to 1 on each file, a loop of 6
  lines with a notch drawn later (#1408). The GlobalId of a void is rvt-rs's
  own, derived from the tag, so the 8 and 7 voids whose tag changes get new
  GlobalIds. The scorer measured is the unfilled-openings one, with the
  property and quantity census, on the two office files (Measure run
  41953415257, head e8ef33d3); Core Interior's and RE1 Architecture's exports
  and the 19 held-out and spent models' are byte for byte `main`'s.

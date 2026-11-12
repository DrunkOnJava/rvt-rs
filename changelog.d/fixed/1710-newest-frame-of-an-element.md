- **Of an element framed in several partitions, the exporter keeps the newest frame, not the one with the greatest height (#1710, RE-444).**
  A Revit 2024 and later project rewrites an edited element into a newer
  partition and leaves the earlier frame in place, and the exporter kept the
  frame with the greatest vertical extent, so an element made shorter since kept
  its older, taller frame. On the BIM4LCA timber office at building permit 414
  elements with a body in Revit's export (251 curtain panels, 65 doors, 52
  mullions, 42 walls and 4 columns) were read from an older frame, the newest
  frame having Revit's height on 405 of them and the frame kept on 1: 3
  mullions and 2 columns had a `Length` 0.05 to 0.123 m too long, and 2 walls
  their old type's `Width`. Within one partition the greater extent still wins.
  Revit 2008 to 2023 already kept the newest frame (RE-107).

- **Of an element framed in several partitions, the exporter keeps the newest frame, not the one with the greatest height (#1710, RE-444).**
  A Revit 2024 and later project rewrites an edited element into a newer
  partition and leaves the earlier frame in place, and the exporter kept the
  frame with the greatest vertical extent, so an element made shorter since kept
  its older, taller frame. On the BIM4LCA timber office at building permit 414
  elements with a body in Revit's export (251 curtain panels, 65 doors, 52
  mullions, 42 walls and 4 columns) were read from an older frame, the newest
  frame having Revit's height on 405 of them and the frame kept on 1: 3
  mullions and 2 columns had a `Length` 0.05 to 0.123 m too long, 20 members
  the type of an older frame, and 2 walls an older type's `Width`. The values of
  that file that differ from Revit's export fall from 18 quantities and 52
  properties to 1 and 1, and the BIM4LCA housing gains 25 quantities equal to
  Revit's. Five values that were equal on the timber office at permit are no
  longer written (two walls now name no type), and one wall is drawn from its
  record box where it was drawn from its saved mesh. Within one partition the
  greater extent still wins. Revit 2008 to 2023 already kept the newest frame
  (RE-107).

- **A wall, floor, roof or ceiling type whose layer copies disagree takes its
  current copy on Revit 2020 too (#1805, RE-466).** It was dropped there, and
  its elements had no layer set. On the one Revit 2020 file measured with such
  a type, the current copy holds Revit's widths and materials on all 5 floors,
  now that a layer with no material of its own takes its category's (RE-464).
  No Autodesk sample of 2015 to 2023 has such a type, so the other releases
  before 2024 still drop it.

- **All 25 Areas of Autodesk's `rac_advanced` sample are written with their
  region on its 2015 to 2024 saves, where 3 were (22 on its 2021 save, where
  none were) (#1261).**
  Where an Area's lines give it no region, its region now comes from Revit's
  own plan topology where it is read:
  for each area scheme on each Level, Revit keeps the loops its area boundary
  lines make, each with its area, and each Area names its loop. The loop is
  rebuilt from the lines, each moved by its area rule's offset (to a wall's
  face, 0.497 ft on that sample) and met where the moved lines cross, arcs
  included, and is used only where its area is the one Revit saves for it
  and the Area's point is inside. That draws the 22 Areas the lines' boxes
  could not: 18 in groups with an arc or a slanted line, and 4 whose lines
  meet only once offset, which earlier notes put down to walls. On the
  BIM4LCA office (2024) the plan topology gives every Area the region its
  lines give it, with the area of Revit's own export to 1e-9, and its export
  is unchanged. An Area whose loop fails a check has no body, as before. No Revit 2015 to 2023 export with
  Areas is available to compare with, so those are checked against the
  areas Revit saves in the file (RE-330).

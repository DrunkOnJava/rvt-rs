- **All 25 Areas of Autodesk's `rac_advanced` sample are written with their
  region on its 2015 to 2020 and 2022 to 2027 saves, where 16 were on
  2015 to 2024 (RE-330) and none on 2025 to 2027; the 2021 save, which holds
  two states of 16 lines, still writes none (#1261).**
  Where an Area's lines give it no region, its region now comes from Revit's
  own plan topology where it is read:
  for each area scheme on each Level, Revit keeps the loops its area boundary
  lines make, each with its area, and each Area names its loop. The loop is
  rebuilt from the lines, each moved by its area rule's offset (to a wall's
  face, 0.497 ft on that sample) and met where the moved lines cross, arcs
  included, and is used only where its area is the one Revit saves for it
  and the Area's point is inside. That draws the 9 Areas the lines' arrangement with its joins could
  not: those with an arc or a slanted line, and those whose lines meet only
  once offset. On the
  BIM4LCA office (2024) the plan topology gives every Area the region its
  lines give it, with the area of Revit's own export to 1e-9; one Area of 30 on each file
  has the same region but two fewer vertices than before, which Revit's
  export has (RE-333 section 8). An Area drawn from the plan topology says
  `SpaceOutlineSource` `area_boundary`, so the 10 that RE-330 marks
  `area_boundary_joined` now say `area_boundary`. Against the
  IFC2X3 export of `rac_advanced` from the three.js repository, which nothing
  of this reader was read off, the 25 Areas of the 2020 save are all within
  0.1% in area and 0.01 ft in outline, none is extra and none missing; the
  16 that the lines' joins draw are equal to their topology loops. The
  export is IFC2X3 by a build of Revit 2021's exporter whose string ends "IU
  alternativa 21.0.0.383" (publisher not established), counted apart, never
  used to remove a value. A group of lines that cannot be read as a whole (a
  design option, a stray height, conflicting copies, a missing line) gives
  no region, the plan topology's included. An Area whose loop fails a check
  has no body, as before, and says why in the diagnostics (RE-333).

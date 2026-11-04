- **More Areas of Revit 2015 to 2024 files are drawn, from area boundary
  lines joined as their records say, and a line off the plan axes refuses
  only the Areas it touches (#1261, RE-330).** An area boundary line often
  stops short of the line it joins (a line drawn along a wall stops at the
  other wall's centre), and two parallel lines can stop at the same place a
  wall's thickness apart. Where two lines name each other, a free end is now
  carried on to the farthest perpendicular line that names it and is named
  by it, and the free ends of two such parallel lines are joined by a short
  connector. An Area whose outline needed either says
  `SpaceOutlineSource` `area_boundary_joined` instead of `area_boundary`. A
  line that is not a straight line along an axis (a slanted line, an arc)
  used to make every Area of its scheme on its Level unreadable; it is now
  left out with its box and refuses only the Areas whose region the box
  touches. On Autodesk's `rac_advanced`, 16 of the 25 Areas are drawn on
  every save but 2021's (3 before); the four once taken to be bounded
  partly by walls are bounded by lines alone, and no wall bounds any Area
  it draws. What this rests on: the only external check is a public IFC2X3
  export of that project from the three.js repository, made by a
  third-party build of Revit's 2021 exporter and showing the model's 2015 to
  2020 state. The 16 Areas drawn on the 2020 save are within 0.1% in area
  and 0.01 ft in outline of it, and none is written that it lacks; the 2022
  and 2023 saves are also checked against the 2024 save's reader. No Revit
  IFC4 export of these Areas exists, and the export is not used to remove a
  value or to settle a disagreement between exporters. The BIM4LCA office's
  30 Areas are unchanged (the IFC is byte for byte as before), and so is the
  held-out BIM Whale project's. Left out: 9 Areas of `rac_advanced` (a
  slanted line, an arc, and a join its records do not name). Report:
  `reports/element-framing/RE-330-area-joins.md`.

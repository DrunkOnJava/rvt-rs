- **More Areas of Revit 2015 to 2024 files are drawn, from area boundary
  lines joined as their records say, and a line off the plan axes refuses
  only the Areas it touches (#1261, RE-330).** An area boundary line often
  stops short of the line it joins, and two parallel lines can stop at the
  same place a wall's thickness apart. Where two lines name each other, a
  free end is now carried on, by at most 1 ft, to the farthest perpendicular
  line that names it and is named by it, when that line spans the place
  reached; and the free ends of two such parallel lines, at most 1 ft apart
  and running away on opposite sides, are joined by a short connector. An
  Area whose outline needed either says `SpaceOutlineSource`
  `area_boundary_joined` instead of `area_boundary`. A line that is not a
  straight line along an axis, or whose record box is wider than the line,
  used to make every Area of its scheme on its Level unreadable; it is now
  left out with its box and refuses only the Areas whose region the box
  touches. On Autodesk's `rac_advanced`, 16 of the 25 Areas are drawn on
  every save but 2021's (3 before), 10 of them `area_boundary_joined`; the
  four once taken to be bounded partly by walls are bounded by lines alone.
  What this rests on: the only external check is an IFC2X3 export from the
  three.js repository, by a build of Revit 2021's exporter whose string ends
  "IU alternativa 21.0.0.383" (publisher not established), showing the
  model's 2015 to 2020 state. The 16 Areas drawn on the 2020 save are within
  0.1% in area and 0.01 ft in outline of it and none is written that it
  lacks, but that is in-sample: the joins were read off this same export, and
  that a reference list means a join is inferred from the fit. The 2022 and
  2023 saves are also checked against the 2024 save's reader. No Revit IFC4
  export of these Areas exists, and the export is not used to remove a value
  or to settle a disagreement between exporters. The BIM4LCA office's 30
  Areas are unchanged (the IFC is byte for byte as before); so is the
  held-out BIM Whale project's, which draws no Area body, so it does not
  exercise the joins. Left out: 9 Areas of `rac_advanced`, 1 to 5 (two
  straight axis lines whose record boxes are 0.497 ft wider than the line),
  7 and 8 (a straight line 0.016 degrees off the axis), 10 (an arc) and 6 (a
  join its records do not name). Report:
  `reports/element-framing/RE-330-area-joins.md`.

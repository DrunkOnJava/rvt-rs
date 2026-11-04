- **Two more Areas of Revit 2015 to 2024 files are drawn, along a boundary
  line that Revit picked from a wall, and the export says why the others are
  not (#1261, RE-332).** An area boundary line picked from a wall names that
  wall in its reference list, and Revit draws it on the wall's outer face
  (exterior walls) or on its centre line (interior walls); the record box
  holds the position. Two lines of Autodesk's `rac_advanced` are drawn on the
  face for one stretch and on the centre line for another, and their record
  keeps the union: a box half the wall's thickness across. Such a line is now
  read as the wall's centre line, and an Area is drawn along it only on a
  stretch more than one wall thickness beyond the host wall's ends, where no
  wall face can move it. That Area says `SpaceOutlineSource`
  `area_boundary_wall_line`; one whose outline runs along the line anywhere
  else has no body and says `wall_line_position_not_proven`. On
  `rac_advanced`, 18 of the 25 Areas are drawn on every save but 2021's (16
  before), 2 of them `area_boundary_wall_line` (Areas 2 and 5). What this
  rests on: the only external check is the IFC2X3 export from the three.js
  repository that RE-330 uses, by a build of Revit 2021's exporter whose
  string ends "IU alternativa 21.0.0.383" (publisher not established), the
  model's 2015 to 2020 state. The rule was read off that export, so the
  agreement is in-sample, and the Areas are labelled for that reason. On the
  2020 save all 18 Areas are within 0.1% in area and 0.01 ft in outline of
  it, none is written that it lacks, and 7 of its 25 are not written. No
  Revit IFC4 export of these Areas exists, and the export is not used to
  remove a value or to settle a disagreement between exporters. Left out:
  Areas 1 and 4, whose outline needs the stretch of the line within its
  wall's span (the export draws the face there on one side of a step and the
  centre line on the other, and nothing in the file says where), and Area 6,
  whose region is Area 4's because a join its records do not name is open (all
  three say `wall_line_position_not_proven`); Area 3, which also needs a
  slanted line; Areas 7, 8 and 10, which have no wall in them. On the
  held-out BIM Whale project the IFC is byte for byte as before (counts
  only, 0 extra Areas); the BIM4LCA office, whose lines are all axis lines,
  reads no wall. Report: `reports/element-framing/RE-332-wall-hosted-lines.md`.

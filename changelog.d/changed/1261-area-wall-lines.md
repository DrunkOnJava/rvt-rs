- **Two more Areas of Revit 2015 to 2024 files are drawn, along a boundary
  line that Revit picked from a wall (#1261, RE-332).** An area boundary line
  picked from a wall names that wall in its reference list, and Revit draws it
  on the wall's outer face (exterior walls) or on its centre line (interior
  walls); the record box holds the position. Two lines of Autodesk's
  `rac_advanced` are drawn on the face for one stretch and on the centre line
  for another, and their record keeps the union: a box half the wall's
  thickness across. Such a line is now read as the wall's centre line, and an
  Area is drawn along it only on a stretch more than one wall thickness
  beyond the host wall's record box, where no wall face can move it. That
  Area says `SpaceOutlineSource` `area_boundary_wall_line`, a new value that
  also appears in the MCP tool descriptions of `space_outline_source`; one
  whose outline runs along the line anywhere else has no body and says
  `wall_line_position_not_proven`. On `rac_advanced`, 18 of the 25 Areas are
  drawn on every save but 2021's (16 before), 2 of them
  `area_boundary_wall_line` (Areas 2 and 5). The 7 not written, by reason:
  `point_in_no_single_region` 4 (Areas 3, 7, 8 and 10: a slanted line and an
  arc), `two_areas_one_region` 2 (Areas 4 and 6, which share one region
  because a join its records do not name is open) and
  `wall_line_position_not_proven` 1 (Area 1, whose outline needs the stretch
  of the line within its wall's span: the export draws the face there on one
  side of a step and the centre line on the other, and nothing in the file
  says where). The 2021 save still draws none. What this rests on: the only
  external check is the IFC2X3 export from the three.js repository that
  RE-330 uses, by a build of Revit 2021's exporter whose string ends "IU
  alternativa 21.0.0.383" (publisher not established), the model's 2015 to
  2020 state. The rule was read off that export, so the agreement is
  in-sample, and the Areas are labelled for that reason. On the 2020 save all
  18 Areas are within 0.1% in area and 0.01 ft in outline of it, none is
  written that it lacks, and 7 of its 25 are not written. The held-out BIM
  Whale project's IFC is byte for byte as before (0 extra Areas, counts
  only), but that check is vacuous: no Area body is drawn there, so none goes
  through the wall rule. No Revit IFC4 export of these Areas exists, and the
  export is not used to remove a value or to settle a disagreement between
  exporters. The BIM4LCA office, whose lines are all axis lines, reads no
  wall. Report: `reports/element-framing/RE-332-wall-hosted-lines.md`.

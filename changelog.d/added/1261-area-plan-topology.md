- **All 25 Areas of Autodesk's `rac_advanced` sample are written with their
  region on its 2015 to 2020 and 2022 to 2027 saves, from Revit's own plan
  topology; 16 were on 2015 to 2024 (RE-330) and none on 2025 to 2027; the
  2021 save, which holds two states of 16 lines, is a separate entry
  (#1261).** For each area scheme on each Level Revit keeps the loops its area
  boundary lines make, each with its area, and each Area names its loop. The
  loop is rebuilt from the lines, each moved by its area rule's offset (to a
  wall's face, 0.497 ft on that sample) and met where the moved lines cross,
  and is used only where its area is the one Revit saves for it and the Area's
  point is inside. It comes first; where it is not read the region is the lines' arrangement with RE-330's joins, and where it is read and fails a check of its own the Area has no body, and a group of lines
  that cannot be read as a whole (a design option, a stray height, a copy that
  names another Level or scheme or lies at another height, a missing line)
  gives no region of either kind; copies that conflict in place give the
  topology's region alone (the 2021 entry). It draws the 9 Areas
  the arrangement could not. An arc is written as an arc, an
  `IfcIndexedPolyCurve` with three-point arcs as Revit's export writes one (a
  span over a half turn as two arcs), and the outline is checked as it is
  written, its area with the arcs as arcs against the circuit's; an arc that
  cannot be read finitely refuses the Area (`area_arc_not_finite`). A point
  where a third line meets a straight outline stays a vertex, as in RE-317.
  An Area drawn from the plan topology says `SpaceOutlineSource` `area_plan_topology` (`area_plan_topology_unchecked` on Revit 2025 to 2027 and on the 2021 save, below), a new value that is also in the MCP tool descriptions
  of `space_outline_source`; on `rac_advanced` the 6 that main marks
  `area_boundary` and the 10 it marks `area_boundary_joined` now say it. An
  Area whose loop fails a check has no body, as before, and says why in the
  diagnostics: `area_not_in_one_plan_topology`, `area_circuit_not_found`,
  `area_circuit_side_not_read`, `area_circuit_area_differs`,
  `area_point_outside_circuit` or `area_arc_not_finite`. Measured on `e9124b3a`
  (Measure runs 41894715743 and 41894722135): against the IFC2X3 export of
  `rac_advanced` from the three.js repository, which nothing of this reader
  was read off, the 25 Areas of the 2020 save are all written, within 0.1% in
  area and 0.01 ft in outline, none extra and none missing, the arc included;
  the 16 that the lines' joins draw are equal to their topology loops on all
  nine saves where main draws them. The export is IFC2X3 by a build of Revit
  2021's exporter whose string ends "IU alternativa 21.0.0.383" (publisher not
  established), counted apart and never used to remove a value. On the
  BIM4LCA office (2024) the 30 Areas have the region, vertices and area they
  had; the IFC differs from main's only in their `SpaceOutlineSource`, in the
  starting vertex of 3 outlines and the order of the holes of 6 (the same
  rings), and in a timestamp. The rule is puzzbobb's (#1371), whose two
  commits keep their authorship; the arcs, the vertex rule and the value
  source are the maintainer's. No Revit IFC4 export of these Areas exists: the
  IFC2X3 export is the only external check, and it is out of sample for this
  rule because the topology's loops and the figures they are checked against
  were not read off it, and only its counts are kept (its terms were never checked); on `5008247d` (Measure run 42017400629) the 2020 save's pairs have 116 spaces in both files, none only in Revit's and none only in rvt-rs's. Where the plan topology and the lines' arrangement both give a
  region they must be the same one, else the Area has no body and says
  `area_regions_disagree`; an Area drawn from the topology has `BodySource`
  `native_area_plan_topology` and a value source `area_plan_topology` that
  names the records it was read from. The counts of what is written are from Measure run 42017400629 on `5008247d`; the vertex-level and equality figures above are from the archived head `e9124b3a` of #1371 (runs 41894715743 and 41894722135) and were not re-read on `bcb02107`.

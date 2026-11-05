- **The elements CSV lists each element on the storey the IFC writes it on,
  and only the elements it writes (#1417).** `rvt-schedule`, Python's
  `schedule_csv()` and the viewer's "Download element schedule (CSV)" took a
  row's `level` from the element's own storey and listed the further pieces
  of an element of separate loops, which the export writes as items of the
  owner's body and not as elements. They now read the writer's own decision,
  `rvt::ifc::step_writer::placements` and `storey_of`, as the MCP server and
  the rooms schedule do (#1412): a part of a stair (a run, landing or
  stringer) or of a curtain wall is on the storey of its whole, an element in
  a room on the storey of the room, an opening on the storey of its host, and
  a folded piece has no row. The `level` and `level_elevation_*` cells are
  empty where the export puts the element in the building only, or where no
  storey is reached through its room, whole or host. An element with folded
  pieces is written with a body of several items, which one width, depth and
  height cannot state, so its size cells are empty: a ceiling of separate
  loops is now one row with empty size cells, where it used to have a row
  for each loop (47 fewer rows on each office file). Rows are still in
  level-elevation order, so a row whose level moves moves in the file. The
  columns do not change. The user guide says what the level column is.
  Measured with `probes/examples/probe_1417_elements_csv_levels.rs` in
  Measure run 41983667731 (code head 38ddc579, `base=none`,
  `scorers=none`), where all 8 models pass the probe's six checks (header,
  cells, count, level-elevation order, every row's ElementId, IFC type and
  level against the placements, and the empty size cells of an element with
  folded pieces) and the model `rvt-ifc` writes has the same written elements
  and storeys as the schedule's model: on the BIM4LCA office as built the
  schedule goes from 13,021 rows to 12,974 and at building permit from 13,067
  to 13,020, the 47 rows left out in each being `IfcCovering` pieces the
  export folds into another element's body, and 599 rows of each change level
  (542 `IfcMember`, 35 `IfcStairFlight` and 22 `IfcSlab`, all parts of a
  whole): 542 from one storey to another and 57 from none to a storey. The 47
  pieces belong to 19 ceilings as built and 20 at permit, none without an
  ElementId, and the width, depth and height of the row of each of them are
  empty. On the Revit 2025 stair models r2025-13, r2025-14 and r2025-15 the
  27 rows stay 27 and 3 take a level (2 `IfcMember`, 1 `IfcStairFlight`), and
  on r2025-16 and r2025-17 the 25 rows stay 25 and 1 `IfcStairFlight` takes
  one; no row moves from a storey to none on any of them, and none of them
  has an element with folded pieces. 2024 Core Interior keeps its 970 rows,
  none moves and none has folded pieces. The office's 12,974 and 13,020 rows
  are the element rows the MCP server's consistency check counted on those
  files in run 41977947391. The IFC text `rvt-ifc` writes is the same on 34
  of 34 models, in run 41983679071 (head 38ddc579 against main 40b86b80,
  `scorers=none`, nordic and r2025 on). No row moved for an element in a
  space or an opening on these models, so those two paths are not exercised
  by this measurement. The held-out models and Autodesk's samples were not
  run, and `rvt-ifc --saved-meshes`, `--family-extrusions` and the viewer's
  saved-mesh path were not measured for folding. The
  diagnostics' count of folded pieces followed in #1424.

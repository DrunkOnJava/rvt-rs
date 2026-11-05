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
  Measure run 41981541756 (code head 6fd13029, `base=none`,
  `scorers=none`), where every model passes the probe's checks (header,
  cells, level-elevation order, every row's ElementId, IFC type and level
  against the placements) and the model `rvt-ifc` writes has the same written
  elements and storeys as the schedule's model: on the BIM4LCA office as
  built the schedule goes from 13,021 rows to 12,974 and at building permit
  from 13,067 to 13,020, the 47 rows left out in each being `IfcCovering`
  pieces the export folds into another element's body, and 599 rows of each
  change level (542 `IfcMember`, 35 `IfcStairFlight` and 22 `IfcSlab`, all
  parts of a whole): 542 from one storey to another and 57 from none to a
  storey. On the Revit 2025 stair models r2025-13, r2025-14 and r2025-15 the
  27 rows stay 27 and 3 take a level (2 `IfcMember`, 1 `IfcStairFlight`), and
  on r2025-16 and r2025-17 the 25 rows stay 25 and 1 `IfcStairFlight` takes
  one; no row moves from a storey to none on any of them. 2024 Core Interior
  keeps its 970 rows and none moves. The office's 12,974 and 13,020 rows are
  the element rows the MCP server's consistency check counted on those files
  in run 41977947391. The IFC text `rvt-ifc` writes is the same on 34 of 34
  models, in run 41981551766 (head 6fd13029 against main b36477f1,
  `scorers=none`, nordic and r2025 on). These figures are those of code head
  6fd13029, before an element with folded pieces had its size cells left
  empty; the count of those rows, and the figures on the head that has that
  change, are not yet measured. The held-out models and Autodesk's samples
  were not run. The viewer's Schedule panel and its info panel still read the
  element's own storey (#1422).

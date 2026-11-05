- **The elements CSV lists each element on the storey the IFC writes it in,
  and only the elements it writes (#1417).** `rvt-schedule`, Python's
  `schedule_csv()` and the viewer's "Download element schedule (CSV)" took a
  row's `level` from the element's own storey and listed the further pieces
  of an element of separate loops, which the export writes as items of the
  owner's body and not as elements. They now read the writer's own decision,
  `rvt::ifc::step_writer::placements` and `storey_of`, as the MCP server and
  the rooms schedule do (#1412): a part of a stair, a railing or a curtain
  wall is on the level of its whole, an element in a room on the level of the
  room, an opening on the level of its host, and a folded piece has no row.
  The `level` and `level_elevation_*` cells are empty only where the export
  contains the element in the building with no storey. Rows are still in
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
  change level (542 curtain-wall members, 35 stair flights and 22 slabs, all
  parts of a whole): 542 from one storey to another and 57 from none to a
  storey. On the Revit 2025 stair models r2025-13, r2025-14 and r2025-15 the
  27 rows stay 27 and 3 take a level (2 `IfcMember`, 1 `IfcStairFlight`), and
  on r2025-16 and r2025-17 the 25 rows stay 25 and 1 `IfcStairFlight` takes
  one; no row moves from a storey to none on any of them. 2024 Core Interior
  keeps its 970 rows and none moves. The office's 12,974 and 13,020 rows are
  the element rows the MCP server's consistency check counted on those files
  in run 41977947391. The IFC text `rvt-ifc` writes is the same on 34 of 34
  models, in run 41981551766 (head 6fd13029 against main b36477f1,
  `scorers=none`, nordic and r2025 on). The held-out models and Autodesk's
  samples were not run. The viewer's Schedule panel and its info panel still
  read the element's own storey (#1422).

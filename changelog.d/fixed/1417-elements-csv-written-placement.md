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
  level-elevation order, so a row whose level moves moves in the file too. The
  columns do not change. The user guide says what the level column is.

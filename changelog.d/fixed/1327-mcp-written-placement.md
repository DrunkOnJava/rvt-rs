- **The MCP server's levels and element lists are the ones the IFC export
  writes (#1327).** The scorer of #1327 found that a part of a stair or a
  railing had no `level` in the server's rows where the IFC puts it on its
  whole's storey, so the levels' element counts differed from the export's (5
  Revit 2025 models, 4 held-out, and the BIM4LCA office, 599 rows of each of
  its two files), and that the office's lists held 47 `IfcCovering` rows the
  export does not write, the further pieces of an element of separate loops
  that it writes as items of the owner's body. Both are the writer's own
  decisions, now `rvt::ifc::step_writer::placements` (folded, a door's or
  window's opening, in a space, part of an aggregate, on a storey, in the
  building) and `storey_of`, which the writer places every element with and
  the server reads: an element's `level` and the levels' `element_count` are
  the storey the export contains it in, through its space, its whole or its
  host, and a folded piece is neither a row nor in `total` or `by_ifc_type`.
  The summary's `elements.folded_into_bodies` counts the folded pieces by IFC
  entity, since the diagnostics' `exported` counts include them. The tool
  output schemas change (`folded_into_bodies`), and so does the contract in
  `docs/mcp-tools.json`. Not yet measured: the Measure runs on this head
  are in the pull request.

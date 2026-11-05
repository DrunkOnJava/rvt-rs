- **The MCP server's levels and element lists are the ones the IFC export
  writes (#1327).** The scorer of #1327 found that a part of a stair, a
  railing or a curtain wall had a level of its own in the server's rows, or
  none, where the IFC puts it on its whole's storey, so the levels' element
  counts differed from the export's (5 Revit 2025 models, 4 held-out, and
  the BIM4LCA office, 599 rows of each of its two files, 542 of them
  curtain-wall members at "Roof" where the IFC says Level_05), and that the
  office's lists held 47 `IfcCovering` rows the export does not write, the
  further pieces of an element of separate loops that it writes as items of
  the owner's body. Both are the writer's own decisions, now
  `rvt::ifc::step_writer::placements` (folded, a door's or window's opening,
  in a space, part of an aggregate, on a storey, in the building) and
  `storey_of`, which the writer places every element with and the server
  reads: an element's `level` and the levels' `element_count` are the storey
  the export contains it in, through its space, its whole or its host, and a
  folded piece is neither a row nor in `total` or `by_ifc_type`. The
  summary's `elements.folded_into_bodies` counts the folded pieces by IFC
  entity, since the diagnostics' `exported` counts include them. The tool
  output schemas change (`folded_into_bodies`), and so does the contract in
  `docs/mcp-tools.json`. Measured with the scorer of #1404 (30480145) and
  `base=none`, on code head 349cce4a (the commits after it are this fragment
  and the contract): run 41954411755 finds no model failing the counts,
  element membership, element storeys or levels checks, which failed on the
  BIM4LCA office as built and at permit (47 coverings, and 599 of 12,974 and
  of 13,020 rows, 542 of them curtain-wall members and the rest stair
  flights and landings) and on the 5 Revit 2025 stair models before (runs
  41949018446 and 41949021964). The models still failing there fail the
  names and type object checks that #1406 fixes, and the office's rooms
  check, which that scorer got wrong for family instances written as spaces
  (9b51c1a8 corrects it). With both fixes merged on a throwaway branch
  (0fd2d2a6, run 41954444137) 30 of 32 models passed every check and the
  other two, the office files, failed only that rooms check; run 41957685928
  (537d88ba, the fixed scorer) re-measures it and is not read yet. IFC
  identity is not established: run 41954408419 (`base=main` at 07caf059)
  found 32 of 34 models identical and the two office files different, and
  this branch is based on 77d71cbb, since when main took two commits that
  change the export of floors; run 41957584014 compares the head with
  77d71cbb itself and is not read yet.

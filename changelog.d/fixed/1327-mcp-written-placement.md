- **The MCP server's levels and element lists are the ones the IFC export
  writes (#1327).** The scorer of #1327 found that a part of a stair or a
  curtain wall had a level of its own in the server's rows, or none, where
  the IFC puts it on its whole's storey, so the levels' element counts
  differed from the export's (5 Revit 2025 models, 4 held-out, and
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
  entity (the diagnostics' `exported` counts held them until #1424). The tool
  output schemas change (`folded_into_bodies`), and so does the contract in
  `docs/mcp-tools.json`. The diagnostics' `storey_bound_elements`, the rooms
  schedule (`room_rows`) and `every_storey_holds_an_element` read
  `placements` too, so a part counts as bound to its whole's storey in all
  of them, and `folded_into_bodies` is always in the summary, empty where
  none is folded. Measured with the scorer of #1404 and `base=none`, on code
  head d0836833 (this change merged with main 6f4bc534, which has #1406):
  run 41965720746 passes 32 of 32 models, the 6 reference models, the 24
  Revit 2025 models and the BIM4LCA office as built and at permit. On main's
  server before, the office failed the counts, element membership, element
  storeys and levels checks (47 coverings, and 599 of 12,974 and of 13,020
  rows, 542 of them curtain-wall members and the rest stair flights and
  landings, 8 and 7 of 10 levels) and the 5 Revit 2025 stair models failed
  element storeys and levels (runs 41949018446 and 41949021964). The IFC
  text `rvt-ifc` writes is the same on 34 of 34 models, the reference
  models, the four office files and the 24 Revit 2025 models, in run
  41965717668 (head d0836833 against main 6f4bc534, `scorers=none`). In the
  diagnostics of that run only `exported.storey_bound_elements` moves, on 7
  of the 34 models: the office as built from 12,961 to 12,971 and at permit
  from 13,008 to 13,018, the Revit 2025 stair models 13 to 15 from 24 to 27
  and 16 and 17 from 24 to 25, the elements bound to a storey through their
  whole. The held-out models and Autodesk's samples were not run.

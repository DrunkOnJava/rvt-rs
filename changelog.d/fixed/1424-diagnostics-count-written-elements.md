- **The diagnostics count the elements the IFC writes, and say how many
  pieces the writer folded (#1424).** `exported.building_elements` and
  `exported.by_ifc_type` counted the further pieces of an element of
  separate loops that the writer folds into the body of the element that
  owns them (#823, `Placement::Folded`), which the IFC does not write as
  elements: Measure run 41977947391 (head 2fc83c47, before this change)
  counted 158 `IFCCOVERING` on the BIM4LCA office as built and 122 at
  permit, where the IFC and Revit's export hold 111 and 75, and 13,021 and
  13,067 building elements where the MCP server lists 12,974 and 13,020.
  They now count what `step_writer::placements` says is written, and the
  new `exported.body_pieces_folded`, by STEP entity type, counts what was
  folded, so a count from before is the new count plus it. A folded piece
  clones its owner's properties, so it also left
  `building_elements_with_geometry`, `body_sources`, `bounding_box_bodies`,
  `bounding_box_spaces`, the refusal and outline counts of `exported` and
  the decisions of `rvt-ifc --export-decisions` that those are totalled
  against; it would otherwise count its owner's source and refusals twice,
  and `body_sources` would no longer add up to `building_elements`.
  `storey_bound_elements` already left it out. The ratios made from these
  counts (`source_coverage.exported_element_fraction` and
  `geometry_element_fraction`) and the element terms of `confidence.score`
  move with them. The IFC is unchanged. The descriptions of the MCP tools
  `open_model` and `get_summary` and the output schema's
  `folded_into_bodies` no longer say that the diagnostics include the
  pieces, so the contract in `docs/mcp-tools.json` changes, and
  `tools/re/mcp_answers_vs_ifc.py` checks the field against the summary's.
  **Upgrading:** a reader of `exported.building_elements`, `by_ifc_type`
  or `body_sources` sees fewer on a file with folded pieces (47 fewer
  `IFCCOVERING` on each BIM4LCA office file, the only measured models with
  one in run 41977947391); add `exported.body_pieces_folded` to get the
  figure from before. Not yet measured on this head.

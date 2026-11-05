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
  folded: the figure of `building_elements` and `by_ifc_type` from before is
  the new one plus it. A folded piece clones its owner's properties and
  storey, so it also left `building_elements_with_geometry`, `body_sources`,
  `bounding_box_bodies`, `bounding_box_spaces`, the refusal and outline
  counts of `exported`, the decisions of `rvt-ifc --export-decisions` that
  those are totalled against, the `skipped` geometry-gap items
  (`unsupported_geometry_*`) and `decoded.parameter_value_count`, which
  counted the `Pset_` values of the sets cloned onto a piece, none of which
  the writer emits; they cannot be recovered from `body_pieces_folded`.
  Otherwise a piece would count its owner's source, refusals and gaps twice,
  and `body_sources` would no longer add up to `building_elements`.
  `storey_bound_elements` already left it out. The ratios made from these
  counts (`source_coverage.exported_element_fraction` and
  `geometry_element_fraction`) and the element terms of `confidence.score`
  move with them. The IFC is unchanged. The descriptions of the MCP tools
  `open_model`, `get_summary` and `export_gltf`, and the output schemas'
  `folded_into_bodies` and `building_elements`, change, so the contract in
  `docs/mcp-tools.json` does: `export_gltf` reports the IFC's counts, and
  now says in `notes` how many pieces its glb draws beyond them.
  `tools/re/mcp_answers_vs_ifc.py` checks the field against the summary's,
  `parameter_value_count` against the `Pset_` values in the IFC, and, with
  `--require-folded-field` on Measure's head side, that the field is there.
  **Upgrading:** a reader of `exported.building_elements` or `by_ifc_type`
  sees fewer on a file with folded pieces (47 fewer `IFCCOVERING` on each
  BIM4LCA office file; none of the other 33 models whose diagnostics run
  41986491801, head f7da9235, kept has one), and `body_sources`, the
  geometry-gap items and `parameter_value_count` change likewise; add
  `exported.body_pieces_folded` to `building_elements` and `by_ifc_type` to
  get their figures from before. Measured on head f7da9235 in run
  41986491801 (`base=main`): the IFC hashes are equal on 53 of 53 models.
  Not yet measured on the head that adds the geometry-gap, `Pset_` and
  `export_gltf` changes.

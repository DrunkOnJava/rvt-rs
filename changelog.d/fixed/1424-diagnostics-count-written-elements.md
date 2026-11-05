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
  41989882391 kept has one), and `body_sources`, the geometry-gap items and
  `parameter_value_count` change likewise; add `exported.body_pieces_folded`
  to `building_elements` and `by_ifc_type` to get their figures from before.
  Measured in Measure run 41989882391 (code head 0b1bc044, `base=main`
  a4044dc8, `nordic`, `r2025` and `holdout` on): `ifc.sha256` is identical
  on all 53 models, and the diagnostics of the 35 models that keep them move
  on the two office files only. As built `building_elements` goes from
  13,021 to 12,974, `building_elements_with_geometry` from 12,787 to
  12,740, `by_ifc_type.IFCCOVERING` from 158 to 111 and
  `decoded.parameter_value_count` from 54,952 to 54,764; at permit from
  13,067 to 13,020, from 12,833 to 12,786, from 122 to 75 and from 55,145
  to 54,957; `body_pieces_folded` is `{"IFCCOVERING": 47}` on both and
  empty on the others, and the `skipped` items do not move on either (no
  piece was in a gap). `parameter_value_count` falls by 188 on each file,
  4 for each of the 47 pieces. On the head side every consistency check of
  the MCP scorer reads 0 differ on all 53 models, with
  `--require-folded-field` on and the new check that
  `parameter_value_count` equals the `Pset_` values in the IFC passing on
  every model; on the base side that check differs on the two office files
  only (the IFC holds 54,764 and 54,957 values, main's diagnostics count
  54,952 and 55,145), so that equality had not been checked before. Not
  measured: the held-out models' diagnostics (their results keep counts
  only), Autodesk's samples, and the `export_gltf` note on a model with
  folded pieces.

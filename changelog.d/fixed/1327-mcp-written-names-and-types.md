- **The MCP server's element names and type objects are the ones the IFC
  export writes (#1327).** The scorer of #1327 found two disagreements on
  every model it ran on. An `IfcSpace`'s `name` in `list_elements` and
  `get_element` was the model's own `Room-<ElementId>`, where the export
  writes the room number as `Name` (or `<room name>:<ElementId>` for a room
  with a name and no number) and the room name as `LongName`; and
  `element_type` was a parallel grouping that lacked the `Other` and
  `Identity Data` sets the writer gives every type object and the
  `IfcSpaceType` it makes for each room, and named a type for elements the
  writer makes none for. Both now come from the writer:
  `rvt::ifc::step_writer::written_names` and `type_objects` are the
  functions it writes the `Name`, `LongName`, type objects and their sets
  with, so the server and the IFC cannot differ on them, and the
  diagnostics' parameter value count of the type objects' sets reads
  `type_objects` too. A row and `get_element` gain `long_name`; `name` is
  absent only for a room with neither a number nor a name whose model name
  is exactly the exporter's `<Revit class>-<ElementId>` placeholder, and the
  placeholder names of other elements are still served as `name` (Einhoven
  23 of 37 rows, RE1 Electrical 3 of 49, the BIM4LCA office as built 52 of
  13,021). No row of the 32 models measured lacks a name, so the absent-name
  path is not measured. `explain_element`'s `name` and
  `other_entities[].name` follow the same rule (they were the model's name).
  `element_type` gains `ifc_type` and `name`, its `element_id` is the type
  object's `Tag` (an `IfcSpaceType`'s is its room's ElementId), and it is
  absent where the writer makes no type object, which it used to be served
  for. The tool output schemas change, and so does the contract in
  `docs/mcp-tools.json`.
  Measured with the scorer of #1404 (its branch head 9fbb594c), all on head
  6c994f30 with `base=none`: run 41951524664 passes every consistency check
  on Einhoven, Core Interior and the four RE1 models, 6 of 6, where run
  41949014871 (the scorer of 1895e771, main's server) failed all 6; run
  41951554180 passes 19 of the 24 Revit 2025 models (1 before, run
  41949021964), the 5 that fail being the stair models, on the level of a
  stair's parts; run 41951550715 passes the office's names, long names, type
  objects and their sets and fails four checks still. As built it lists 158
  `IfcCovering` rows against the export's 111, and 599 of 12,974 rows and 8
  of 10 levels' counts differ; at permit, 122 coverings against 75, 599 of
  13,020 rows and 7 of 10 levels. The IFC text `rvt-ifc` writes is unchanged
  by the move: in run 41951527615 (`base=main` at 77d71cbb, `scorers=none`)
  the `ifc.sha256` of 34 of 34 models, the reference models, the four office
  files and the 24 Revit 2025 models, is identical. The held-out models and
  Autodesk's samples were not run.

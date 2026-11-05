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
  Measured with the scorer of #1404 (30480145), on head 34ace109 with
  `base=none`: run 41954686452 passes 25 of 32 models. The 6 reference
  models (Einhoven, Core Interior, RE1) pass, where run 41949014871 (the
  1895e771 scorer, main's server) failed all 6, and 19 of the 24 Revit 2025
  models pass (1 before, run 41949021964); the 5 that fail are the stair
  models, on the level of a stair's parts and the levels' counts. The
  BIM4LCA office as built and at permit fails counts, element membership,
  element storeys and levels: as built 158 `IfcCovering` rows against the
  export's 111, 599 of 12,974 rows and 8 of 10 levels' counts; at permit 122
  against 75, 599 of 13,020 rows and 7 of 10 levels. It also failed the
  rooms check, which that scorer got wrong for the 44 family instances
  written as spaces (9b51c1a8 corrects it; run 41957587403 on this branch's
  tip re-measures it). Its names, long names, type objects and their sets
  are equal. The IFC text `rvt-ifc` writes is unchanged by the move: in run
  41954681945 (`base=main` at 07caf059, `scorers=none`) the `ifc.sha256` of
  34 of 34 models, the reference models, the four office files and the 24
  Revit 2025 models, is identical. The held-out models and Autodesk's
  samples were not run.

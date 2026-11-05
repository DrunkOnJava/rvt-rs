- **The MCP server's element names and type objects are the ones the IFC
  export writes (#1327).** The scorer of #1327 found two disagreements on
  every model it ran on. An `IfcSpace`'s `name` in `list_elements` and
  `get_element` was the model's own `Room-<ElementId>`, where the export
  writes the room number as `Name` (or `<room name>:<ElementId>` for a room
  without one) and the room name as `LongName`; and `element_type` was a
  parallel grouping that lacked the `Other` and `Identity Data` sets the
  writer gives every type object and the `IfcSpaceType` it makes for each
  room, and named a type for elements the writer makes none for. Both now
  come from the writer: `rvt::ifc::step_writer::written_names` and
  `type_objects` are the functions it writes the `Name`, `LongName`, type
  objects and their sets with, so the server and the IFC cannot differ. A
  row and `get_element` gain `long_name`, and `name` is absent for a room
  with neither a number nor a name (the model's own name for it is a
  placeholder made from its class and ElementId, which no source named, and
  is no longer served as a name). `element_type` gains `ifc_type` and
  `name`, and is absent where the writer makes no type object. The tool
  output schemas change, and so does the contract in `docs/mcp-tools.json`.
  Measured with the scorer of #1404 (its branch head 9fbb594c), all on head
  6c994f30 with `base=none`: run 41951524664 passes every consistency check on
  Einhoven, Core Interior and the four RE1 models, 6 of 6, where run
  41949014871 failed all 6 on the server before; run 41951554180 passes 19 of
  the 24 Revit 2025 models (1 before), the 5 that fail being the stair models,
  on the level of a stair's parts (a flight and its railing members have none
  where the IFC puts them on their whole's storey); run 41951550715 passes
  the office's (as built and at permit) names, long names, type objects and
  their sets and fails four checks still: the counts and the element
  membership (47 `IfcCovering` rows the export does not write), the storeys of
  599 of 12,974 rows and 8 of 10 levels' counts. The IFC text `rvt-ifc`
  writes is unchanged by the move: in run 41951527615 (`base=main` at
  77d71cbb, `scorers=none`) the `ifc.sha256` of 34 of 34 models, the reference
  models, the four office files and the 24 Revit 2025 models, is identical.
  The held-out models and Autodesk's samples were not run.

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
  The IFC text `rvt-ifc` writes is unchanged by the move (not yet measured
  on this head).

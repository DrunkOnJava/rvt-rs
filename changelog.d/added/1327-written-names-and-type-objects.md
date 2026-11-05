- **`rvt::ifc::step_writer::written_names` and `type_objects` (#1327).**
  `written_names(ifc_type, name, type_guid, property_set)` returns the
  `Name` and `LongName` the IFC writer writes for a building element (an
  `IfcSpace`'s room number and room name, or `<room name>:<ElementId>`
  without a number) and whether the name is the exporter's own `<Revit
  class>-<ElementId>` placeholder. `type_objects(model)` returns the type
  objects the writer makes (`TypeObjects::groups`, one per type, entity and
  predefined type of the elements named `Family:Type`, and `rooms`, one
  `IfcSpaceType` per room whose name was read), each with its entity,
  `Name`, `Tag`, property sets and the entities it types. `write_step`
  writes with both, and the diagnostics' count of the type objects'
  parameter values reads `type_objects`, which leaves out the pieces folded
  into another element's body; a reader that wants an element's name or type
  object asks here instead of grouping again.

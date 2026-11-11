- **An element still drawn as its record's box says why its saved mesh was not
  used (#1645).** After the pass that draws a box from the mesh Revit saved for
  it (Revit 2023 to 2027), each box that stays one has a `SavedMeshRefusal`
  property in its `RvtElementRecordGeometry` set: `no_saved_graphics`,
  `no_saved_mesh`, `no_triangles`, `element_id_shared`, or `partial: ` and the
  decoder diagnostics that left the mesh partial (`unsupported reachable
  graphics class ...`, `shared curved edge ...`). The diagnostics count them by
  reason in `exported.saved_mesh_refusals`, and the decision log
  (`rvt-ifc --export-decisions`, the MCP server's `explain_element`) names the
  reason of each element as `saved_mesh_not_used`. No body is drawn
  differently.

- **An `IfcPlateType` is written with no `Tag` (#1594).** Revit's export gives a
  curtain panel's type object the `Tag` of one of its plates, the first of its
  group in an order of the curtain grid that no record holds (RE-367 section
  4a; on Vitoria's `ALD26 Before` the 67 plates make 5 types and each `Tag` is
  the first plate of its group). rvt-rs wrote the panel type's own ElementId,
  which is not Revit's `Tag` on any of them, so the 269 plates of the BIM4LCA
  office at building permit (260 as built) each disagreed. The type's own
  ElementId is still its `Type Id` property and the MCP server's `element_id`
  for it; members and furniture, whose `Tag`s agree, are unchanged.

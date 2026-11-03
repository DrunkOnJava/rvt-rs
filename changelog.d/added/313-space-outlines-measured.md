- **Every `IfcSpace`'s plan area and outline is scored against Revit's export,
  and says where its outline came from.** Measure's new
  `space_profiles_vs_ifc` scorer pairs the spaces of rvt-rs's export with
  Revit's and counts those within 0.1% in area and 0.01 ft in outline (outline to outline, and by the mesh's vertices); the
  IfcOpenShell CI job runs it on Core Interior (116 of 116 rooms) and RE1
  Architecture (11 of 11) and fails below those counts. A room's
  `RvtElementRecordGeometry.SpaceOutlineSource` is `stored_solid` where its
  outline was read from the room's stored solid and `record_box` where the
  body is the record's bounding box and its plan area is the box's, not the
  space's (`saved_mesh` with `--saved-meshes`, `family_body` for a family
  instance or Revit Space drawn as more than its box, `none` for an Area,
  which has no body). A family instance written as a space gets the property
  too. `exported.bounding_box_spaces` in the diagnostics and a line in
  `rvt-ifc`'s summary count the spaces drawn as their record's box. What an
  `IfcSpace` holds per release, and what is measured where, is in
  [docs/rooms.md](docs/rooms.md) (#1260, RE-313).

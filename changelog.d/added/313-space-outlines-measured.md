- **Every `IfcSpace`'s plan area and outline is scored against Revit's export,
  and says where its outline came from.** Measure's new
  `space_profiles_vs_ifc` scorer pairs the spaces of rvt-rs's export with
  Revit's and counts those within 0.1% in area and 0.01 ft in outline; the
  IfcOpenShell CI job runs it on Core Interior (116 of 116 rooms) and RE1
  Architecture (11 of 11) and fails below those counts. A room's
  `RvtElementRecordGeometry.SpaceOutlineSource` is `stored_solid` where its
  outline was read from the room's stored solid and `record_box` where the
  body is the record's bounding box and its plan area is the box's, not the
  room's (`saved_mesh` with `--saved-meshes`, `none` for an Area, which has no
  body). `exported.bounding_box_spaces` in the diagnostics and a line in
  `rvt-ifc`'s summary count the rooms drawn as their box. What an
  `IfcSpace` holds per release, and what is measured where, is in
  [docs/rooms.md](docs/rooms.md) (#1260, RE-313).

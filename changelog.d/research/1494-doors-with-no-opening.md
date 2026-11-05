- **A door set in no wall is placed at its stored origin (#1494, RE-361).**
  Where Revit's export gives a door no opening, its `IfcLocalPlacement` origin
  is the origin of its stored transform (RE-357); rvt-rs now writes it there
  for a door it binds to no host wall, whose stored transform is read and
  upright. The door's body, a saved mesh or the record box, stays where it was
  in the world, and no GlobalId, host or opening moves. What does move is the
  location those doors report: `location_feet`, so their location in the
  elements CSV, the scene graph and the MCP server's views goes from the
  record box's centre to Revit's own placement origin, and the viewer's
  schedule with them. The doors carry
  `RvtElementRecordGeometry.PlacementOriginSource`, `stored_origin_no_opening`
  (the MCP server lists it among their trust markers), and the decoded Door
  record gains `m_opening_placement_origin_x`, `_y` and `_z`. Measured on this
  change's Measure run once it has finished.

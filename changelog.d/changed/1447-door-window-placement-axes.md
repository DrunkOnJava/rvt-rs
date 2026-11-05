- **A door or window set in a plain wall is placed on Revit's axes, and its
  `IfcLocalPlacement` says which way it faces (#1447, RE-347).** rvt-rs
  wrote the default axes (X along the model's +X, Z up) on every door and
  window, which were Revit's on 115 of the 521 doors and 226 of the 540
  windows it binds to a plain wall in Core Interior, RE1 Architecture and
  the BIM4LCA office as built and at permit (Measure run 42066757105), and
  it writes no `OperationType`, so the placement's +Y was all the file said
  of the direction. Now the placement's X and Y are the stored axes,
  reversed where `m_hand_flipped` differs from `m_facing_flipped` (RE-343),
  Z up and the origin unchanged, and
  `RvtElementRecordGeometry.PlacementAxesSource` says so. Revit's axes are
  the same on all 1,061 doors and windows so placed in those models, on all
  24 doors and windows of the R2025 models and on all 145 of the 2024 to
  2027 held-out models' (counts only), and every door's and window's body
  and opening is where it was in world coordinates, as saved meshes and as
  record boxes, with its GlobalId, type GlobalId and host unchanged (a
  quarter turn swaps the two sides of its box; Measure runs 42106711251 and
  42106731054). So `rotation_deg` of the elements CSV and the viewer's
  schedule, the Rotation row of the viewer's info panel and
  `rotation_degrees` of the MCP server's `get_element` are now set for every
  such door and window not on +X (0, 90, 180 or -90), and on one whose body
  is its box `width_ft` and `depth_ft` (the panel's Width and Depth) swap on
  a quarter turn: width is now along the placement's X, the door's width,
  where it was the model's X extent. Core Interior's export gains 138
  `PlacementAxesSource` values and 98 directions. A door bound to no plain
  wall, an element with no stored transform (Revit 2023 and older) and an
  instance turned off the model's axes keep the default axes. On Revit 2025,
  2026 and 2027 the same elements are held to the 2024 save of the same
  project (`tools/re/bodies_across_saves.py`): 124, 96 and 82 doors and
  windows of two Autodesk sample projects, 80, 58 and 45 of them off the
  default axes, are the same, and 28 and 42 the 2024 save places stay on the
  default axes in the 2026 and 2027 saves of `rac_advanced` (counted, not
  scored). The origin is still the record box's centre, more than 0.5 ft
  from Revit's on almost all (#1465). Not measured: Snowdon Towers against
  Revit's export, and the memory of the added read of the stored transforms
  (its time is below what a run resolves).

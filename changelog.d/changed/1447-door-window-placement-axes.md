- **A door or window set in a plain wall is placed on Revit's axes, and its
  `IfcLocalPlacement` says which way it faces (#1447, RE-347).** rvt-rs wrote
  the default axes (X along the model's +X, Z up) on every door and window,
  which were Revit's on 115 of the 521 doors and 226 of the 540 windows it
  binds to a plain wall in Core Interior, RE1 Architecture and the BIM4LCA
  office as built and at permit (Measure run 42066757105), and it writes no
  `OperationType`, so the placement's +Y was all the file said of the
  direction. Now the placement's X and Y are the stored axes, reversed where
  `m_hand_flipped` differs from `m_facing_flipped` (RE-343), Z up and the
  origin unchanged, and `RvtElementRecordGeometry.PlacementAxesSource` says so.
  Revit's axes are the same on all 1,061 doors and windows so placed in those
  models and on all 145 of the 2024 to 2027 held-out models' (Measure run
  42092391898, counts only), and every door's and window's body and opening
  is where it was in world coordinates, with its GlobalId, type GlobalId and
  host unchanged (a quarter turn swaps the two sides of its box). A door
  bound to no plain wall, an element with no stored transform (Revit 2023
  and older) and an instance turned off the model's axes keep the default
  axes. On Revit 2026 and 2027 the same elements are held to the 2024 save
  of the same project (`tools/re/bodies_across_saves.py`, run 42092410697):
  96 and 82 doors and windows of two Autodesk sample projects, 58 and 45 of
  them off the default axes, are the same. The origin is still the record
  box's centre, more than 0.5 ft from Revit's on almost all (#1465). Not
  measured: Snowdon Towers, the R2025 models, and the time of the added
  read of the stored transforms.

- **A door's and a window's hand flip and facing flip are decoded
  (#1357, RE-343).** The decoded Door and Window records carry
  `m_hand_flipped` and `m_facing_flipped`, the `m_flippedX` and `m_flippedY`
  booleans of the instance's current native `FamilyInstance` record, on Revit
  2024 to 2027, and the typed Door view's `flip_hand` and `flip_facing` read
  them (they were null on every door; not yet measured on this head). No door or window is stored as a
  reflection: the first orthonormal block of its element data has determinant
  1 on all 1,439 doors and windows scored against Revit's export (Measure run
  42014420180 at f65fc702, and Snowdon Towers scored on a Mac) and on all
  1,820 of the Autodesk samples of 2024 to 2027 (run 42024111444 at
  e2f8accc), so the transform holds neither flip. `m_opening_facing` is that
  transform's Y axis: a half turn from the unflipped one where
  `m_hand_flipped` is set, and unchanged by `m_facing_flipped`. For a door or
  window set in a plain wall rvt-rs binds, Revit's own placement is the
  stored axes reversed where the two flips differ, so the direction it faces
  (the placement's +Y, where IFC4 has the leaf open for a family whose swing
  lies on its +Y side) is `m_opening_facing` reversed there. That is exact on
  all 653 such doors and 626 such windows of Core Interior, RE1
  Architecture, the BIM4LCA office as built and at permit, the R2025 models
  and Snowdon Towers, and on 41 doors and 105 windows of held-out models
  (counts only). The two office files are one building and the R2025 models
  one layout six times: counted once, 456 doors and 346 windows. The case
  where the flips differ (287 doors and 286 windows, 186 and 146 counted
  once) is measured on three 2024 buildings, one 2025 door, one 2026 house
  and no 2027 element. The 160 doors rvt-rs binds to no plain wall follow the
  rule on 111 and not on 49, and are not claimed. Within one symbol, a left or
  right `OperationType` of Revit's export differs between two doors exactly
  when the flips differ, on 161 of 162 pairs as built, 161 of 161 at permit
  and 108 of 108 on Snowdon Towers. The fields are carried, equal to the
  booleans, by 1,820 of the 1,820 doors and windows of the Autodesk samples
  of 2024 to 2027 and by none of 2015 to 2023, whose doors and windows are not
  decoded as Door and Window records (run 42024111444). On 2024 and 2025 the
  flips are read in the pass that already reads the doors' and windows'
  symbols and add none; on 2026 and 2027 they add one. Against `main` the IFC
  is byte for byte the same on all 111 models of the unfiltered run
  (42024095189) and, over 64 files, no peak memory moves by more than 1 MiB
  and export time by more than 1.2% on the files of 2023 to 2025 or 1.6% on
  those of 2026 and 2027 (run 42024099050), where the same measurement varies
  by 7% on files the change does not touch.

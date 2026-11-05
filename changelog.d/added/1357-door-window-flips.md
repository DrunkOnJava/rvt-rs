- **A door's and a window's hand flip and facing flip are decoded
  (#1357, RE-343).** The decoded Door and Window records carry
  `m_hand_flipped` and `m_facing_flipped`, the `m_flippedX` and `m_flippedY`
  booleans of the instance's native `FamilyInstance` record, wherever a
  record is read. No door or window is stored as a reflection: the first
  orthonormal block of its element data has determinant 1 on all 1,439 doors
  and windows scored against Revit's export (Measure run 42014420180 at
  f65fc702, and Snowdon Towers scored on a Mac) and on all 1,820 of the
  Autodesk samples of 2024 to 2027 (run 42015182768 at d8b2d2b5), so the
  transform holds neither flip. `m_opening_facing` is that transform's Y
  axis: a half turn from the unflipped one where `m_hand_flipped` is set, and
  unchanged by `m_facing_flipped`. The direction a door faces is
  `m_opening_facing` reversed where the two flips differ, and it is Revit's
  own placement's Y on all 653 doors and 626 windows whose host wall's record
  is read, over Core Interior, RE1 Architecture, the BIM4LCA office as built
  and at permit, the R2025 models and Snowdon Towers; 379 of those 813 doors
  and 286 of the 626 windows are in the case where the two differ, and every
  door state is populated in Core Interior, RE1 Architecture, the office and
  Snowdon Towers. Within one symbol, a left or right `OperationType` of
  Revit's export differs between two doors exactly when the flips differ, on
  161 of 162 pairs as built, 161 of 161 at permit and 108 of 108 on Snowdon
  Towers. The 160 doors with no host wall record read follow the placement
  rule on 111 and not on 49, and are not claimed. Revit's IFC is not
  changed. The fields are carried, equal to the booleans, by 1,820 of the
  1,820 doors and windows of the Autodesk samples of 2024 to 2027 and by none
  of 2015 to 2023, whose doors and windows are not decoded as Door and
  Window records (run 42021284465 at 7d346f1f). Not yet measured on this
  head: the fields, and the time and memory of reading them in the pass that
  reads the doors' and windows' symbols (runs named in the pull request).

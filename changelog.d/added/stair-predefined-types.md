- **A stair's PredefinedType is the one Revit's export gives it, from its
  runs and landings (#323).** Every stair was `.NOTDEFINED.` or unset. A
  stair with a spiral run is now `SPIRAL_STAIR`, one straight run
  `STRAIGHT_RUN_STAIR`, two runs and a landing `TWO_STRAIGHT_RUN_STAIR`,
  `QUARTER_TURN_STAIR` or `HALF_TURN_STAIR` as the second turns, and three
  or four runs each square to the one before `DOUBLE_RETURN_STAIR` or
  `THREE_QUARTER_TURN_STAIR`, on Revit 2024 and 2025. On the BIM4LCA office
  all 16 stairs of each file have Revit's predefined type, where 2 did, and
  so do the 5 R2025 stair models; ten more of the office's stair type
  objects take Revit's GlobalId with it.

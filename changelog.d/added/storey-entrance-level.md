- **Storeys' `Pset_BuildingStoreyCommon.EntranceLevel` (W7, RE-275).**
  revit-ifc's definition of the set gives `EntranceLevel` no calculator, so
  Revit's export reads a yes/no parameter named
  `Pset_BuildingStoreyCommon.EntranceLevel`, then `EntranceLevel`, on the
  Level. rvt-rs writes the value the Level's own record holds, where it holds
  one: on the BIM4LCA office, as built and at building permit, 9 storeys (the
  Level_01 yes, 8 more no) and none for Sea Level, which holds no value, each
  Revit's. A file whose Levels declare no such parameter, as the R2025
  models', gets none.

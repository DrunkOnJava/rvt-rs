- **A category's material is read on Revit 2015 to 2023 where its object
  style has no pattern (#1805, RE-464).** That entry holds a null object where
  a patterned one holds its `PatternHelper`, so the material sits 10 bytes
  earlier, and none was read. It is every host category of Autodesk's
  `rac_advanced`, `rme` and `rst` samples of 2015 to 2023. A wall, floor, roof
  or ceiling layer there that takes its category's material now carries that
  material, as `rac_basic`'s layers already did.

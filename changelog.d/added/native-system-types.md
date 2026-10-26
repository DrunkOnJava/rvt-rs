- **Pipes, wall sweeps and ramps take the type their own record names
  (RE-240, #35).** Where neither RE-130's stored pipe-type layout nor RE-69's
  reference list types one, a pipe's `RbsPipeCurve.m_idType`, a wall sweep's
  `WallSweep.m_typeId` or a ramp's `Ramp.m_rampAttrId` names its type, and the
  type's record its name. On both BIM4LCA office files the 12 pipes, 9 wall
  sweeps and 5 ramps that had neither now carry Revit's Name, ObjectType and
  type object, and 52 more of Revit's property values (their types'
  `Reference`) are written, none different.

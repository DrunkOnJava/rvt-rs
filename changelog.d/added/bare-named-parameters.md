- **Parameters named for a standard property alone.** A project or shared
  parameter named for a standard property set's property alone (`FireExit`)
  now fills that property in each set that applies, where no
  `<set>.<property>` parameter does, as Revit's export reads them: the
  BIM4LCA office's `Pset_DoorCommon.FireExit` on all 266 doors, where none
  was written before (RE-227).

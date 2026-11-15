- **A door or window of a Revit 2015 to 2023 file that the reference-list join
  leaves untyped is typed from its own native record (#1603, RE-477).** The scan
  names a type only through a symbol record with a name entry. On the spent
  Revit 2018 project 68 of the 70 windows name only their master symbol there,
  and Revit's own type is an id the scan knows nothing of. The instance's native
  record names it (`InstanceInfo.m_symbolId`), its master symbol the type's name
  and the family. A member of a model group takes its master symbol as the type,
  as Revit's export does for the spent project's 2 grouped doors, and is left
  untyped where its symbol is not a copy of that master. On the spent project
  the 70 windows and 68 doors now have Revit's Name, `ObjectType` and type
  object (0 of 70 windows before), with the type's `Pset_WindowCommon.Reference`
  and `Pset_DoorCommon.Reference`; on a held-out Revit 2020 project the 4
  windows and 9 doors that differed are equal, and on BIM Whale's Revit 2021
  project 115 doors are (126 of 126 now). On that project 10 of the 115 doors
  take a `Pset_DoorCommon.Reference` that Revit's IFC2X3 export does not write
  (a residual of the `Reference` rule, 1 door of 11 before). Their type objects'
  GlobalIds are not Revit's, and they take no material from the type map yet.

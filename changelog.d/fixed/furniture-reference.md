- **Furniture's `Reference`.** buildingSMART's IFC4 definitions list
  `Reference` in `Pset_FurnitureTypeCommon` with no type, as they do in three
  other sets, and the property-set table left it out. The table generator now
  gives such a property the type most sets give it (`IfcIdentifier`), and
  lists every other property a set defines, so each piece of furniture
  carries its type's name there, as Revit's export does: all 98 of the
  BIM4LCA office's as built and at building permit.

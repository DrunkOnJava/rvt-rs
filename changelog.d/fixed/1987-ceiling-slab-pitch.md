- **A ceiling exported as `IfcSlab` no longer carries `Pset_SlabCommon.PitchAngle`
  0 (#1987).** RE-238's level-slab rule is measured on floors, roofs and stair
  landings; on the BIM4LCA timber housing Revit's export writes no `PitchAngle`
  on a compound ceiling it exports as a slab.

- **Floors' `NetVolume` (#1016, RE-295).** Revit's export writes a floor's
  `Qto_SlabBaseQuantities.NetVolume` as the volume of its solid: its
  `GrossArea` times the total thickness of its type's layers, flat or
  sloped, unless something cuts it. A floor whose native record holds
  neither a `CutPairGStep` nor a `WallCutoutGStep` carries that. On the
  BIM4LCA office that is 31 of the 63 `NetVolume` values Revit's export
  writes, as built and at building permit, none different and none that
  Revit's export leaves out. The volumes of the stair landings Revit draws
  as a mesh and of the roofs written as slabs are not written.

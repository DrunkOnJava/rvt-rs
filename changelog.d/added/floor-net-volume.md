- **Floors' `NetVolume` and `Width` (#1016, RE-295).** Revit's export
  writes a floor's `Qto_SlabBaseQuantities.NetVolume` as the volume of its
  solid: its `GrossArea` times the total thickness of its type's layers,
  flat or sloped, unless something cuts it. A floor whose native record
  holds neither a `CutPairGStep` nor a `WallCutoutGStep` carries that, and,
  where its type has more than one layer, its `Width`, the same thickness.
  On the BIM4LCA office that is 31 of the 63 `NetVolume` values and 14 of
  the 34 `Width` values Revit's export writes, as built and at building
  permit, none different and none that Revit's export leaves out. A
  single-layer floor's `Width`, the volumes of the stair landings Revit
  draws as a mesh and of the roofs written as slabs are not written.

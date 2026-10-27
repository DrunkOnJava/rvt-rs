- **Floors' `Depth` and `GrossVolume` (#1016).** A floor drawn as one
  extrusion whose type has one layer, as deep as the extrusion, carries
  `Qto_SlabBaseQuantities.Depth`, the extrusion's length, and `GrossVolume`,
  its profile's area times that length, as Revit's export writes them: 13 of
  the BIM4LCA office's floors as built and at permit, each Revit's value,
  none different. A layered floor gets neither: Revit's export gives one
  layer's depth there. Its `Perimeter` is left out, as Revit's export writes
  it in feet, unscaled, in a millimetre file; the census counts it apart on
  each slab where its value, read as feet, is Revit's own outline (13 of the
  office's floors at each stage).

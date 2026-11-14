- **A sloped floor drawn along the plane its shape-edited corners lie in gets
  its `Qto_SlabBaseQuantities.Depth` and `GrossVolume` on Revit 2024 (#1208, RE-298,
  RE-384).** Other releases keep neither until a file of theirs is
  measured. Revit's export writes the volume of the slab there, the type's
  layers' total thickness times the cosine of the slope for the `Depth` and
  that times the floor's `GrossArea` for the `GrossVolume`, on all 8 sloped
  floors of the BIM4LCA office as built, where rvt-rs wrote
  neither. A level floor of several layers keeps neither: Revit 2024's
  exporter writes its last layer's thickness and Revit 26.4's another value
  (RE-298).

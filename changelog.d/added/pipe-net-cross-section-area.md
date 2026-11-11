- **A pipe segment gets `Qto_PipeSegmentBaseQuantities.NetCrossSectionArea`, the
  section of the cylinder rvt-rs draws it as (#1016, RE-247).** Revit's export
  gives it on all 12 pipe segments of the BIM4LCA office as built and it is that
  pipe's `GrossCrossSectionArea` on every one, since Revit draws a pipe as a
  solid cylinder with no bore in that section.

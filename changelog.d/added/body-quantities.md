- **Base quantities from a stair flight's, pipe's and column's body (#1016,
  RE-247).** Each stair flight drawn from its sketch carries
  `Qto_StairFlightBaseQuantities.Length`, the span of its riser lines, and
  `NetVolume`; each pipe whose cylinder is read carries its
  `GrossCrossSectionArea` and `OuterSurfaceArea`; and each column joined to
  nothing carries its `Length`, each as Revit's export writes it. On the
  BIM4LCA office the quantities equal to Revit's go from 19,972 to 20,157 as
  built, none different and none that Revit's export leaves out. A joined
  column's `Length` and a curtain panel's `Width` are left out: Revit's
  export gives none on 18 meshed columns and 3 panels that nothing in the
  model tells apart.

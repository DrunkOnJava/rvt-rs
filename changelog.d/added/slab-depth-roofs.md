- **Level floors' `Depth` and flat roofs' slab quantities (#1016, RE-298).**
  Revit's export writes a slab's `GrossVolume` as its `GrossArea` times its
  `Depth` on all 42 of the BIM4LCA office's slabs that have a volume. On a
  level floor of several layers (10, of the types `VP-1`, `VP-2` and `AP-1`)
  `Depth` is the last layer's thickness, not the extrusion's, and on a flat
  roof Revit's export writes as a slab (10) it is the layers' total. rvt-rs
  writes both, with the roof's `GrossArea`, `GrossVolume` and, where its
  record holds no cutting step, `NetVolume`: the roof a hatch cuts keeps
  Revit's smaller volume unwritten. As built and at building permit, none
  different and none that Revit's export leaves out. A roof's `Width` is its
  profile's extent and stays unwritten.

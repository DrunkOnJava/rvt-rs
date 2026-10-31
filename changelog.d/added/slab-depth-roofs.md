- **Flat roofs' slab quantities (#1016, RE-298).** Revit's export writes a
  flat roof as a slab, and gives it `Qto_SlabBaseQuantities`: its `GrossArea`
  (its sketch's profile's area), its `Depth` (the layers' total, however
  many there are) and `GrossVolume` (the area times that depth), and its
  `NetVolume` where its record holds no cutting step. rvt-rs writes them on
  all 10 of the BIM4LCA office's flat roofs, as built and at building
  permit, none different and none that Revit's export leaves out; the roof a
  hatch cuts keeps Revit's smaller `NetVolume` unwritten, and a roof's
  `Width` (its profile's extent) stays unwritten. A level floor of several
  layers gets no `Depth` or `GrossVolume`: the office's export (Revit
  24.0.5) gives it its last layer's thickness, but Revit 26.4's export of
  the held-out Revit 2026 house writes another value on 2 such slabs, so
  rvt-rs leaves it out.

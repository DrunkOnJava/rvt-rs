- **A roof-joined wall's `Height` (#1046, RE-293).** A wall joined to roofs,
  floors or footings that lie clear of it, which gets `GrossSideArea`,
  `NetSideArea`, `GrossVolume` and `NetVolume` from its record's box (RE-270,
  RE-290), now also carries `Qto_WallBaseQuantities.Height`, the body's:
  Revit's export writes it on exactly those walls, and rvt-rs's extrusion is
  as tall as Revit's solid on every one. 47 more walls of the BIM4LCA
  office as built and 58 at building permit carry it, Revit's on every
  one, none different and none that Revit's export leaves out: the office
  goes from 28,278 to 28,492 of the 39,388 quantities equal as built
  (Measure run 41183229365).

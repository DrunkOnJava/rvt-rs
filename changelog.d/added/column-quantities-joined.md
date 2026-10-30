- **A joined column's areas and volumes (#1046, RE-291).** A column a clean
  pair joins and rvt-rs draws as its record's box carries the box's
  `CrossSectionArea`, `OuterSurfaceArea`, `GrossVolume` and `NetVolume` when
  every slab or beam its cut and clean pairs name lies wholly above or below
  it and no wall or covering cuts it. Revit's export draws a column as a cut
  mesh, whose quantities are not the box's, exactly where a slab its cut pair
  names lies across an end of it. They are Revit's on all 186 such columns of
  the BIM4LCA office as built and 250 at building permit, none different and
  none that Revit's export leaves out: the office goes from 28,065 to 28,809
  of the 39,388 quantities equal as built (Measure run 41146562639). A
  joined column's `Length`, a column no clean pair joins that is drawn as its
  box, and the 18 meshed columns stay unwritten.

- **A box-drawn column's areas and volumes (#1046, RE-291).** A column
  rvt-rs draws as its record's box, whose type is a plain rectangle (its
  parameters `b` and `h`, or `b` alone, are the box's sides), carries the
  box's `CrossSectionArea`, `OuterSurfaceArea`, `GrossVolume` and
  `NetVolume` when no clean pair joins it, or when every slab its cut and
  clean pairs name lies wholly above or below it and no wall or covering
  cuts it. Revit's export draws a column as a cut mesh, whose quantities
  are not the box's, exactly where a slab its cut pair names lies across an
  end of it; a round column's type holds a `Diameter` and gets none. They
  are Revit's on all 199 such columns of the BIM4LCA office as built and
  265 at building permit, none different and none that Revit's export
  leaves out: the office goes from 28,065 to 28,861 of the 39,388 quantities equal
  as built (Measure run 41147469913). A joined column's `Length`, the
  round column and the 18 meshed columns stay unwritten (#1203).

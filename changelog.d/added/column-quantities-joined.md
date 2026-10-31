- **A box-drawn column's areas and volumes (#1046, RE-291).** A column
  rvt-rs draws as its record's box, whose type is a plain rectangle (its
  length parameters are exactly `b` and `h`, or `b` alone, the box's sides),
  carries the box's `CrossSectionArea`, `OuterSurfaceArea`, `GrossVolume` and
  `NetVolume` when its pair steps are all read and every slab they name lies
  wholly above or below it, with no wall or covering cutting it, whether or
  not a clean pair joins it. On the BIM4LCA office (Revit 2024, the one
  building measured), Revit's export draws a column as a cut mesh, whose
  quantities are not the box's, exactly where a slab its cut pair names lies
  across an end of it; a round column's type holds a `Diameter` and gets
  none. They are Revit's on all 199 such columns of the office as built and
  265 at building permit, none different and none that Revit's export
  leaves out: the office goes from 28,162 to 28,958 of the 39,388 quantities equal
  as built (Measure run 41183092458). A joined column's `Length`, the
  round column and the 18 meshed columns stay unwritten (#1203).

- **A round column no longer carries a square's quantities (#1203, RE-416).**
  24 round columns of Autodesk's `rac_advanced` sample (450 mm, 6 in each save
  of Revit 2024 to 2027) hold a type whose one length is `b`, which the column
  quantity pass read as a square section, so each was written with the
  `CrossSectionArea`, `OuterSurfaceArea`, `GrossVolume` and `NetVolume` of the
  square that bounds it, 4/pi of the cylinder's. A column drawn from its saved
  mesh now keeps those four only where its box's volume is within 2% of the
  mesh's, and keeps its `Length`. No Revit export holds those saves to score the
  24 against; every model that has one is byte-identical.

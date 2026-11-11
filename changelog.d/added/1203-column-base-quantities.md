- **A column drawn from its saved mesh gets the `Length`, `CrossSectionArea`,
  `OuterSurfaceArea` and `GrossVolume` Revit's export writes for an extruded
  column (#1203, RE-424).** `Qto_ColumnBaseQuantities` now holds them, read from
  the mesh Revit saved for the column where that is one closed solid of flat faces
  with no hole through it, a prism along the vertical with two flat, square ends of
  one area whose volume is the end area times its extent (the rule of a beam's
  quantities, RE-423, with the axis the vertical): the extent, the area of one end,
  the lateral area and the volume. They are written where the column's set holds
  none of `CrossSectionArea`, `OuterSurfaceArea` and `GrossVolume`, or all three and
  no `Length`, and no value a column holds is replaced. A column Revit draws as a
  cut mesh where a slab meets its top, which has a `CrossSectionArea` and a
  `NetVolume` and no more, is no prism along the vertical and gets none, nor do a
  round column, a hollow section or a column drawn as its record box. On the
  BIM4LCA timber office as built they read equal to Revit's on 340 of 344
  `CrossSectionArea`, 339 of 339 `OuterSurfaceArea`, 338 of 339 `GrossVolume` and
  339 of 339 `Length`, from 60, 59, 58 and 203, and at permit on 336 of 340, 335 of
  335, 334 of 335 and 333 of 335, from 58, 57, 56 and 197, none different; the
  concrete office at permit gains the four on 6 columns, the housing's concrete
  files 3 of 7 (each state), its timber files 2 of 54 and its terrain files 1 of 5,
  and a held-out Revit 2023 model 4 of its 8 columns, each equal. Autodesk's
  `rst_basic` of 2023 to 2027 and Snowdon Towers Architectural and Structural gain
  them on 44 columns, which no Revit export scores. Measured on one project with a
  Revit export (the BIM4LCA office and housing, Revit 2024); the housing's 50
  box-drawn columns get none (#1705). The change reads no graphics the saved-mesh
  pass did not read: Measure's timing of 64 files reads 0.98 to 1.04 in time and
  1.00 to 1.04 in peak memory, the spread the 30 samples of 2015 to 2022, which the
  pass cannot read, show too. `--no-saved-meshes` runs no pass, so it leaves these
  quantities out.

- **A column drawn as its type's rectangle gets the `Length`, `CrossSectionArea`,
  `OuterSurfaceArea`, `GrossVolume` and `NetVolume` its saved mesh gives (#1705,
  RE-425).** The BIM4LCA housing draws 50 of its timber file's 54 columns (and 4
  of the concrete file's 7) as the rectangle their type's section gives, a body
  neither RE-291's box rule nor the saved-mesh pass took, though Revit's export
  writes the four on every one. The mesh Revit saved for such a column is now read
  for its quantities only, as a beam's and a wall's are, and where it is one closed
  solid of flat faces that is a prism along the vertical (RE-424) they are written,
  creating `Qto_ColumnBaseQuantities` where the column holds none; no value a column
  holds is replaced, and the body stays the rectangle. On the housing they read
  equal to Revit's on 52 of the timber file's 54 `CrossSectionArea`,
  `OuterSurfaceArea` and `GrossVolume` (from 2) and 54 of 54 `NetVolume` and
  `Length` (from 4 and 25), 7 of 7 of the concrete file's and 5 of 5 of each terrain
  file's, 526 values on six files, none different, the four columns of a chamfered
  concrete family reading Revit's section and not the rectangle's. Core Interior
  (107 columns, whose Revit export holds no base quantities) and Autodesk's
  `rme_advanced` and `rme_basic` of 2024 to 2027 (4 struts each, whose section is
  not their type's rectangle) gain them too, which no Revit export scores; the
  office's exports are byte-identical. Reading these columns' graphics costs memory
  on some files (Measure run 43000953082, base and head timed in turn on one runner,
  three rounds): Core Interior peaks at 536 MiB where it peaked at 437 to 453, the
  two timber housing files at 37 MiB more (1.04 times), and no other file moves
  outside the runs' noise. `--no-saved-meshes` runs no pass, so it leaves these
  quantities and this cost out.

- **What Revit's wall areas and volumes are (RE-270, #1046).** In revit-ifc
  24.1 and 24.2 (the office's exporter) a wall's `GrossSideArea` is the
  largest vertical face's outer boundary and `NetSideArea` the largest face
  with its holes out, the solid's volume is `NetVolume`, and `GrossVolume` is
  the wall type's width times the `GrossSideArea`. Worked out of Revit's own
  bodies they give 435 and 430 of the office's 579 walls as built (grouping
  the faces by normal, as 25.2 does, gives 270 and 276) and all 138 of the
  R2025 models'; `GrossFootprintArea` is `Length` times `Width` on all 279
  walls that carry it. Revit writes the four on exactly the walls it writes
  a `Height` on, and `NetVolume` on every wall, 28 more as built.
  `tools/re/wall_quantities_vs_ifc.py` is the measurement.

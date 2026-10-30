- **A wall's side areas and volumes (#1046, RE-270).** A wall whose native
  record holds no step that cuts, embeds or sweeps it and whose body is its
  record's box cut back at its joins, a rectangle, carries
  `Qto_WallBaseQuantities.GrossSideArea`, `NetSideArea`, `GrossVolume` and
  `NetVolume`, as revit-ifc's `CreateWallBaseQuantities` works them out of
  the wall's solid: the longer side's area, the rectangle times the height,
  and the wall type's width times the side area. They are Revit's on all 49
  of the BIM4LCA office's such walls as built, 48 at building permit and 91
  of puzzbobb's R2025 walls, none different and none that Revit's export
  leaves out; the office as built goes from 27,752 to 27,948 of the 39,721
  quantities equal, at permit from 27,686 to 27,878 and the R2025 models
  from 411 to 775 (Measure run 41108951821). Revit writes the four on
  exactly the walls it writes a `Height` on, and a `NetVolume` on the rest;
  the walls with a top join, openings, cut pairs, embedded walls or sweeps
  stay unwritten.

- **A wall whose extra faces are the jambs of an opening rvt-rs cuts with
  nothing filling it can get its side areas, gross volume and height (#1046).**
  The rule for a wall that is a prism less its openings counted only the
  openings of the doors and windows it hosts. A rectangular opening cut through
  the wall with no filler (a wall opening, a door's opening in a second wall)
  now counts too. A recess, which stops inside the wall, does not, and neither
  does an outline that is not a rectangle.
  On the BIM4LCA housing and office (two projects, eight files), 745 more of the
  walls' `GrossSideArea`, `NetSideArea`, `GrossVolume` and `Height` values equal
  Revit's, with no value different and none Revit's export does not write.

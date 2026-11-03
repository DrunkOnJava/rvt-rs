- **A room whose stored solid the exact walk did not close is read again
  from the faces its solid lists, resolved to a boundary, and labelled
  apart.** The walk stopped at a record longer than 0x200 bytes between two
  faces, walls on lines up to 0.0015 ft apart left a step no face crosses, a
  column overlapping a wall put a loop partly inside the wall's, and a solid
  ran past the 64 KB window. The solid ends at the count of its list of
  objects and at a gap of 1,024 bytes, so the faces of the element that
  follows are not read, and the work is bounded (512 faces, 4,096 pieces;
  crafted solids take under 10 ms). `SpaceOutlineSource` is
  `stored_solid_faces` for these rooms, and `stored_solid_approximate` where
  a column's back leaves the outline up to 0.02 ft short of the record box;
  `stored_solid` stays the exact walk's. The diagnostics count them
  (`exported.resolved_outline_spaces`, `exported.approximate_outline_spaces`)
  and the room schedule leaves an approximate room's area blank. On the
  BIM4LCA office as built the 13 rooms that kept their box take an outline.
  Ten are `stored_solid_faces`: nine with Revit's area to 0.1% (their boxes
  were 3% to 67% over it) and room 1671995, 323% over it (its box, 329%;
  Revit's export splits it into 9 pieces). Three are
  `stored_solid_approximate`, the two Revit's export pairs 0.68% and 0.47%
  under its area. At building permit 11 of 12 are `stored_solid_faces`, 10
  with Revit's area to 0.1% and room 1671995 350% over, and room 3095831
  keeps its box. 6 of the 8 rooms that kept their box do on a held-out
  Revit 2021 project, all 6 at Revit's area. The six `rac_advanced` rooms
  with round columns, 49 of the 50 `rme` rooms and Snowdon Towers' seven keep
  their box. Core Interior and RE1 Architecture are unchanged. A resolved
  room carries no `NetVolume` (#1260, RE-316).

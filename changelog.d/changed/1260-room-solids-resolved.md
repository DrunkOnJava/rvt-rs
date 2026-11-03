- **A room whose stored solid the exact walk did not close is read again
  from the faces its solid lists, and labelled apart.** The walk stopped at a
  record longer than 0x200 bytes between two faces, walls on lines up to
  0.0015 ft apart left a step no face crosses, a column overlapping a wall
  put a loop partly inside the wall's, and a solid ran past the 64 KB window.
  The solid ends at the count of its list of objects and at a gap of 1,024
  bytes, so the faces of the element that follows are not read, and the work
  is bounded (512 faces, 4,096 pieces; crafted solids take under 10 ms).
  `SpaceOutlineSource` is `stored_solid_faces` where the faces' edges close as
  the walk reads them, and `stored_solid_approximate` where a column overlaps
  a wall (its area depends on whether Revit's export takes the column off) or
  the outline falls up to 0.02 ft short of the record box; `stored_solid`
  stays the exact walk's. The diagnostics count both
  (`exported.resolved_outline_spaces`, `exported.approximate_outline_spaces`)
  and the room schedule leaves an approximate room's area blank. On the
  BIM4LCA office as built the 13 rooms that kept their box take an outline:
  nine are `stored_solid_faces`, all at Revit's area to 0.1% (their boxes were
  3% to 67% over it), and four `stored_solid_approximate`, 0.45% to 0.68%
  under Revit's floor area. At building permit 11 of 12 take an outline, all
  `stored_solid_approximate`, ten at Revit's floor area to 0.1%; room 3095831
  keeps its box. 6 of the 8 rooms that kept their box do on a held-out Revit
  2021 project, all 6 at Revit's area. Measure's `space_profiles_vs_ifc`
  compares the faces of Revit's mesh that point up, which read a room under a
  stepped or sloped ceiling as a fraction of itself (room 1671995: 1218 of
  5177 ft²; RE-313's 66% room); the new `space_overlaps_vs_ifc` also compares
  the floor, where all 443 office `stored_solid` room scores agree. The six
  `rac_advanced` rooms with round columns, 49 of the 50 `rme` rooms and
  Snowdon Towers' seven keep their box. Core Interior and RE1 Architecture are
  unchanged. A resolved room carries no `NetVolume` (#1260, RE-316).

- **A room whose stored solid the walk did not close takes the outline of
  the faces up to the solid's end instead of its record's box, where those
  close on the box.** The walk stopped at a record longer than 0x200 bytes
  between two faces, walls on lines up to 0.0015 ft apart left a step no
  face crosses, a column overlapping a wall put a loop partly inside the
  wall's, a solid ran past the 64 KB window, and a cylinder's face (a round
  column) was read as a plane. On the BIM4LCA office as built 12 of the 13
  rooms that kept their box take an outline, and of the 11 that Revit's
  export pairs 9 have its area to 0.1% and 2 are 0.5% and 0.7% off (the
  solid has a column Revit's export does not take off); at building permit
  10 of the 12 take an outline, all 10 with Revit's area to 0.1%. Their
  boxes were up to 329% over Revit's area. The six `rac_advanced` rooms
  with round columns close in every release of the sample, which has no
  Revit export to score them against; office room 1671995 and permit room
  3095831 and 49 of the 50 `rme` rooms keep their box, each for a measured
  reason. Core Interior and RE1 Architecture are unchanged. An outline
  read this way carries no `NetVolume` (#1260, RE-316).

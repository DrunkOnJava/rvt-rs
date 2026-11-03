- **A room whose stored solid the walk did not close takes the outline of
  the faces up to the solid's end instead of its record's box, where those
  close on the box.** The walk stopped at a record longer than 0x200 bytes
  between two faces, walls on lines up to 0.0015 ft apart left a step no
  face crosses, a column overlapping a wall put a loop partly inside the
  wall's, and a solid ran past the 64 KB window. On the BIM4LCA office as
  built 9 of the 13 rooms that kept their box take an outline and all 9
  have Revit's area to 0.1% (their boxes were 3% to 67% over it); at
  building permit 10 of the 12 do, all 10 to 0.1%. Rooms whose outline
  would stand short of their box, whose span runs into other elements,
  whose solid holds a round column or whose floors and ceilings step keep
  their box: three more office rooms as built, two at permit, the six
  `rac_advanced` rooms with round columns and 49 of the 50 `rme` rooms,
  each for a measured reason. Core Interior and RE1 Architecture are
  unchanged. An outline read this way carries no `NetVolume` (#1260,
  RE-316).

- **A wall's top is cut at the floor that covers it (RE-267, #1067).** A wall
  whose top join or Join Geometry pair names a slab, roof or footing is drawn
  up to that element's underside when the element's outline covers the whole
  of the wall's plan, clear of its holes and openings, as Revit's export draws
  it: on the BIM4LCA office 64 of the 65 tops Revit cuts as built and 70 of 70
  at building permit are now Revit's, no other wall's base or top moves, and
  the exports of Core Interior, RE1 Architecture and the R2025 models are
  byte-identical (Measure run 41092512565). A wall that a floor covers in part
  keeps its full height, where Revit steps the top (29 office walls as built
  and 26 at permit), and so does the one wall as built whose plan lies
  0.0243 ft² outside the floor and which Revit cuts all the same. A cut wall
  carries `BodyHeightSource` `wall_top_cut_at_covering_floor` and
  `TopCoverElementId`, the element it is cut at, and no `Height`, which
  Revit's export omits on 2 of them as built and 4 at permit.

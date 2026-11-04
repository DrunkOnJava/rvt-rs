- **A wall a floor covers in part is drawn stepped (RE-320, #1067).** Revit
  cuts a wall's top at the underside of the floor, roof or footing its top
  join or a Join Geometry cut pair names, over the part of the wall's plan
  that element covers, and leaves the wall's own top over the rest; rvt-rs
  drew that only where one element covers the whole plan (RE-267). A wall
  such elements cover in part is now one `SweptSolid` body: its own plan up
  to the lowest underside and, over the part of the plan they do not cover,
  convex pieces up to the next underside or the wall's top. On the BIM4LCA
  office that is 49 walls as built and 46 at building permit. Against
  Revit's own export, rvt-rs's top equals Revit's over the whole plan on 583
  of 607 walls as built (544 before) and 571 of 589 at permit (530), and the
  area drawn at a wall's full height where Revit's body is cut falls from
  1,915.8 to 238.2 ft2 as built and from 1,888.1 to 38.1 ft2 at permit; no
  wall that was equal moved, the 64 and 70 walls cut whole, every wall's
  lowest and highest point and the quantity, property set, layer and opening
  scores are unchanged, and Core Interior, RE1 Architecture and the R2025
  models export byte for byte as before. A stepped wall carries
  `BodyHeightSource` `wall_top_stepped_at_covering_floor` and
  `TopCoverElementId`, and gets no `Height`, side areas or volumes, as
  before. The glTF and the plan sheet still draw it in its layers and by its
  stepped plan.
- **An element that only a cut pair names cuts a wall's top only where it
  reaches it (RE-320, #1067).** A Join Geometry cut takes the volume the two
  elements share, which is the wall's top only where the element rises to it.
  Three office walls as built (3484876, 3490093, 3490551) pass through a slab
  whose top is 0.984 ft below theirs, and Revit leaves their tops whole;
  the rule that cut a wall at any element it names would have cut them. The
  64 and 70 walls RE-267 cuts whole are unchanged, and a cut-pair element
  that reaches the wall's top (20 walls as built, 10 at permit) still cuts.
- **Not drawn, and measured.** A wall whose layers Revit ends at different
  heights (the KS walls 1347198 to 1347201, taller in rvt-rs by 102.9, 14.3,
  35.7 and 11.7 ft2 as built), a top Revit cuts at a height no join names,
  and the gaps and ends Revit's walls have where no floor reaches. One wall as
  built (3004978) keeps its full height: 0.0243 ft2 of its plan, all of it
  inside the wall it abuts, is cut by Revit, and no width, area or position
  of that piece separates it from the pieces Revit keeps, 0.032 ft slivers
  included (they are drawn). The room schedule leaves a stepped wall's
  Width, Depth and Height columns empty, since its body is no longer one
  extrusion, and the GLB's stepped tops are not scored.

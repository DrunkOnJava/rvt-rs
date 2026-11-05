- **A floor whose corners are shape-edited is drawn along its sloped top, and
  the walls that stand on it start from that top (#1384, RE-336).** A floor's
  native `SlabShapeEditGStep` moves its corner vertices; where they lie in one
  plane the floor is the slab between that plane and the same surface one
  thickness lower (`BodySource` `native_slab_shape_edit`), where its record box
  reproduces the plane's highest point. A wall whose base join
  (`JoinToRoofGStep`, base) names one element that covers the whole of its plan
  is cut at that element's top over the plan, as a solid between that top and
  the wall's own (`BodyBaseSource` `wall_base_sloped_at_covering_floor`, or
  `wall_base_cut_at_covering_floor` for a level element), with the element in
  `BaseCoverElementId`. Where this is not proven the element keeps its record
  box or the wall its base, and the reason is a trust marker on the element
  (`SlabShapeRefusal`, `BaseCoverRefusal`): edits of interior or edge vertices
  or creases, corners not in one plane, an outline in pieces, a wall covered
  in part, by two elements or with the top crossing its base or top. An edited floor, roof or ceiling is never taken for a flat one, and a floor whose edit is
  found but not read leaves the walls on it as they were. The refusals are counted in
  `exported.slab_shape_refusals` and `exported.wall_base_refusals` and are
  decisions of the log (`slab_shape_refused`, `wall_base_cut_refused`). The top
  cut (RE-267) no longer takes a floor drawn along a sloped top for a level
  one. On the BIM4LCA office the 8 such floors, as built and at permit, have
  Revit's volume to 0.01%, and 12 of the 14 Basement walls Revit raises as
  built (the 4 on level floors and 8 of the 10 on sloped footings; 12 of 16 at
  permit) have Revit's lowest point (598 of 607 walls with
  Revit's base and top, from 586; 577 of 589 at permit, from 565); the other
  2 (4) are 0.123 ft (0.044 ft) off, from 0.656 ft, because their plans end
  short of Revit's (RE-337 then brings 2057668 and 2057670 to Revit's lowest
  point on both files). No wall's top moved, and no other model's export moves
  but for the refusal markers on Autodesk's Snowdon Towers (18) and `rac_advanced`
  (1 a save), which have no redistributable Revit IFC4 export to score them against. Details:
  `reports/element-framing/RE-336-sloped-footings.md`.

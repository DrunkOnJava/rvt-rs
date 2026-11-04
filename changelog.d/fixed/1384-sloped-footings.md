- **A floor whose corners are shape-edited is drawn along its sloped top, and
  the walls that stand on it start from that top (#1384, RE-336).** A floor's
  native `SlabShapeEditGStep` moves its corner vertices; where they lie in one
  plane the floor is the slab between that plane and the same surface one
  thickness lower (`BodySource` `native_slab_shape_edit`), where its record box
  reproduces the plane's highest point. A wall whose base join
  (`JoinToRoofGStep`, base) names one element that covers the whole of its plan
  is cut at that element's top over the plan, as a solid between that top and
  the wall's own (`BodyHeightSource` `wall_base_sloped_at_covering_floor`, or
  `wall_base_cut_at_covering_floor` for a level element), with the element in
  `BaseCoverElementId`. Where this is not proven the element keeps its record
  box or the wall its base, and the reason is a trust marker on the element
  (`SlabShapeRefusal`, `BaseCoverRefusal`): edits of interior or edge vertices
  or creases, corners not in one plane, an outline in pieces, a wall covered
  in part, by two elements or with the top crossing its base or top. The top
  cut (RE-267) no longer takes a floor drawn along a sloped top for a level
  one. The measured effect is in `reports/element-framing/RE-336-sloped-footings.md`.

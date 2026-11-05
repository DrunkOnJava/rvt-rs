- **A wall with an edited elevation profile, cut back by its join from inside
  the wall beneath it, is drawn to that wall's face (#1396, RE-337).** The
  Basement walls of the BIM4LCA office that stand on the sloped footings of
  RE-336 were cut back, by the join of their record, to the face of a Level_01
  wall that overlaps them by 0.66 ft of 9.5; Revit's body at the base runs on
  to the face of the Basement wall beneath, which the record box's end lies
  inside. A wall with an edited profile that is a plain rectangle along a model
  axis, drawn from its record box, now has an end drawn to the face of the
  perpendicular wall beneath it that holds its base (the wall stands on it, not
  on its top) and holds the box's end, where it stands beyond the end by no
  more than its own thickness, no such wall touches the end and the cut of the
  wall's base still resolves over the longer plan. The added end is a block
  drawn up to the lowest of the top of the wall it reaches, the base of any
  other wall the pass reads that it would lie inside (a wall turned off a
  quarter turn or drawn as another kind of solid is not read), and the wall's
  own top. **This is a labelled approximation** (Griffin, 2026-11-05: ship it,
  labelled): Revit steps the end up further and follows a wall at each height
  by a join order that is not read, so the steps above are not drawn, and a
  wall whose block stops below its top carries `BodyEndSource`
  `wall_end_to_base_wall_upper_step_omitted` (`wall_end_extended_to_base_wall`
  only where the block reaches the wall's full height, which no measured wall
  does) and `EndCoverElementId`; the wall's profile openings stay where they
  were and the join trims stay the record's. A wall that keeps an end the rule
  would have drawn carries `BodyEndRefusal`, counted in
  `exported.wall_end_refusals` and logged as `wall_end_extension_refused`. Two
  of its five reasons (`wall_beneath_not_one_rectangle`,
  `base_not_cut_over_the_extended_plan`) have measured instances;
  `base_cut_lost_on_extended_plan`, `walls_beneath_reach_the_end_at_different_faces`
  and `wall_stands_in_the_added_end` have none, and the stepped body of a wall
  with no base cut, the Level branch of the blocks, the high end of a wall,
  walls along x, an end reaching the wall's full height, the plane-offset term
  of the opening shift, a stepped-top wall beneath whose lowest underside is at
  or below the candidate's base, two candidates at one corner and a candidate
  with no base join are exercised by no measured wall, so nothing has checked
  them against a Revit export. A beneath wall drawn as another kind of solid is
  neither reached nor refused. Walls 2057668 and 2057670 have Revit's lowest
  point as built and at building permit (0.123 ft and 0.044 ft off before),
  their volume goes from 1.7 and 1.3 ft3 under Revit's to 0.6 and 0.1 under (as
  built; 0.4 under to 0.1 over at permit, where the block itself is 0.1 ft3
  under and the rest is excess the short end masked), and no wall has more
  points above Revit's top in the plan rows of `wall_top_cover_vs_ifc` than
  before. Walls 2058499 and 2058501 keep their ends: as built no footing covers
  the longer plan, and at permit the wall beneath them is drawn with a stepped
  profile, so they stay 0.044 ft off there. No other wall of either office file,
  Core Interior, RE1 Architecture, the 24 R2025 models, the 58 Autodesk sample
  projects or the 25 held-out models changes (Measure 41980282001, 41980285452
  and 41980288900 at `1b3a6bd7`; the same comparisons at `b47a9a00`, runs
  41978479893, 41978482673 and 41978485557, agree). This corrects the 0.15.0 entry for #1384
  (RE-336), which says that 2 walls as built (4 at permit) are 0.123 ft
  (0.044 ft) off Revit's lowest point because their plans end short of Revit's:
  after this, 2057668 and 2057670 have it on both files and only 2058499 and
  2058501 at permit stay off, by 0.044 ft. The measured effect is in
  `reports/element-framing/RE-337-wall-ends-at-the-base-wall.md`.

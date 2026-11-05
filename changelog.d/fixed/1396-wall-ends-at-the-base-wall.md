- **A wall with an edited elevation profile, cut back by its join from inside
  the wall beneath it, is drawn to that wall's face (#1396, RE-337).** The
  Basement walls of the BIM4LCA office that stand on the sloped footings of
  RE-336 were cut back, by the join of their record, to the face of a Level_01
  wall that overlaps them by 0.66 ft of 9.5; Revit's body at the base runs on
  to the face of the Basement wall beneath, which the record box's end lies
  inside. A wall with an edited profile that is a plain rectangle along a model
  axis, drawn from its record box, now has an end drawn to the face of the
  plain perpendicular wall beneath it that holds its base (the wall stands on
  it, not on its top) and holds the box's end, where it stands beyond the end
  by no more than its own thickness, no such wall touches the end and the cut
  of the wall's base still resolves over the longer plan. The added end is a
  block drawn up to the top of the wall it reaches and no higher, since Revit
  steps the end up further, and the wall's profile openings stay where they
  were. It carries `BodyEndSource` and `EndCoverElementId` (the join trims stay
  the record's). A wall that keeps an end the rule would have drawn carries
  `BodyEndRefusal`, counted in `exported.wall_end_refusals` and logged as
  `wall_end_extension_refused`; two of its four reasons
  (`wall_beneath_not_one_rectangle`, `base_not_cut_over_the_extended_plan`)
  have measured instances, and `base_cut_lost_on_extended_plan` and
  `walls_beneath_reach_the_end_at_different_faces` have none, so nothing has
  exercised them against a Revit export, and no measured wall took the
  end without a base cut, so that path of the stepped body has none either; a
  beneath wall drawn as a solid is neither reached nor refused. Walls 2057668
  and 2057670 have
  Revit's lowest point as built and at building permit (0.123 ft and 0.044 ft
  off before), their volume goes from 1.7 and 1.3 ft3 under Revit's to 0.1 and
  0.6 over (0.4 under to 0.2 over at permit) and the area where their top is
  higher than Revit's stays at 0.34 ft2. Walls 2058499 and 2058501 keep their
  ends: as built no footing covers the longer plan, and at permit the wall
  beneath them is drawn with a stepped profile, so they stay 0.044 ft off
  there. No other wall of either office file, Core Interior, RE1 Architecture,
  the 24 R2025 models, the 58 Autodesk sample projects or the 25 held-out
  models changes (Measure 41957511578, 41957514477 and 41957517612 at
  `1b417c45`). The measured effect is in
  `reports/element-framing/RE-337-wall-ends-at-the-base-wall.md`.

- **A wall with an edited elevation profile, cut back by its join from inside
  the wall beneath it, is drawn to that wall's face (#1396, RE-337).** The
  Basement walls of the BIM4LCA office that stand on the sloped footings of
  RE-336 were cut back, by the join of their record, to the face of a Level_01
  wall that overlaps them by 0.66 ft of 9.5; Revit's body at the base runs on
  to the face of the Basement wall beneath, which the record box's end lies
  inside. A wall with an edited profile that is a rectangle along a model axis,
  drawn from its record box, now has an end drawn to the face of the
  perpendicular wall beneath it that holds its base and holds the box's end,
  where it stands beyond the end by no more than its own thickness, no such
  wall touches the end and the cut of the wall's base still resolves over the
  longer plan. It carries `BodyEndSource` and `EndCoverElementId`. A wall that
  keeps an end the rule would have drawn carries `BodyEndRefusal`, counted in
  `exported.wall_end_refusals` and logged as `wall_end_extension_refused`.
  Walls 2057668 and 2057670 have Revit's lowest point as built and at building
  permit (0.123 ft and 0.044 ft off before); walls 2058499 and 2058501 as built
  keep their ends, since no footing covers the longer plan; and no other wall
  of either office file, Core Interior, RE1 Architecture, the 24 R2025 models,
  the 58 Autodesk sample projects or the 25 held-out models changes (Measure
  41950456463, 41950459960 and 41950462786 at `33529d0b`). Revit steps the end
  up the wall's height and rvt-rs draws one prism, so the area where the top of
  the two walls is higher than Revit's grows by 0.24 ft2 each (0.07 ft2 at
  permit), and walls 2058499 and 2058501 at permit stay 0.044 ft off, their
  wall beneath drawn with a stepped profile. The measured effect is in
  `reports/element-framing/RE-337-wall-ends-at-the-base-wall.md`.

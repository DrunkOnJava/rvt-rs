- **A wall with an edited elevation profile, cut back by its join from inside
  the wall beneath it, is drawn to that wall's face (#1396, RE-337).** The
  Basement walls of the BIM4LCA office that stand on the sloped footings of
  RE-336 were cut back, by the join of their record, to the face of a Level_01
  wall that overlaps them by 0.66 ft of 9.5; Revit's body at the base runs on
  to the face of the Basement wall beneath, which the record box's end lies
  inside. A wall with an edited profile that is a plain rectangle along a model
  axis, drawn from its record box, with a join trim, now has its end drawn to
  the faces of the walls its record joins there, where the cut of the wall's
  base still resolves over the longer plan (RE-336, #1411). A wall that keeps
  an end the rule would have drawn carries `BodyEndRefusal`, counted in
  `exported.wall_end_refusals` and logged as `wall_end_extension_refused`. A
  rule that did not ask the record's joins drew a Snowdon Towers soffit wall
  0.1016 m longer than Revit's. This corrects the 0.15.0 entry for #1384
  (RE-336), which says that 2 walls as built (4 at permit) are 0.123 ft
  (0.044 ft) off Revit's lowest point because their plans end short of Revit's:
  walls 2057668 and 2057670 have it as built and at permit, and 2058499 and
  2058501 with #1409 and #1411. How the end steps up the height is in the entry
  for #1410 (RE-348). The measured effect is in
  `reports/element-framing/RE-337-wall-ends-at-the-base-wall.md` and
  `reports/element-framing/RE-348-wall-end-join-order.md`.

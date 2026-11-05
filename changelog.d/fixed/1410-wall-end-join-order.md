- **A profile-edited wall's end follows, at each height, the first wall its
  record joins there that stands (#1410, #1409, RE-348).** Revit steps the
  ends of the BIM4LCA office's Basement walls 2057668, 2057670, 2058499 and
  2058501 up the wall's height, to the face of the Basement wall beneath, then
  of a wall between it and Level_01, then of Level_01's. Which of the walls
  that stand at a height it follows is the order in which the wall's record
  lists the walls joined at that end (`JoinEndGStep.m_realJoined`; the order of
  `m_geomJoined`, the last wall that stands and the least ElementId each give a
  different end on at least three of the four walls). **That order rests on 4
  walls in one project.** The end is now drawn as the wall's own plan to its
  top and one column from its base for each distance the end reaches beyond the
  next, each to the height the walls it follows stand to, from the plans of
  those walls: a plain rectangle, a layered profile, or a wall whose top a
  floor cut in steps, read by its pieces up to its own top. The labelled
  approximation of #1396 (`BodyEndSource`
  `wall_end_to_base_wall_upper_step_omitted`, which omitted the steps above the
  wall beneath) is retired; a wall carries `BodyEndSource`
  `wall_end_stepped_to_joined_walls` or keeps the end its record gives with a
  counted reason. A kept end carries `BodyEndRefusal` (`joined_wall_not_read`,
  `joined_wall_top_not_read`, `joined_wall_does_not_meet_the_end`,
  `joined_walls_do_not_reach_the_wall_height`,
  `joined_wall_face_inside_the_wall`, `joined_wall_faces_overhang_the_lower_end`,
  `base_cut_lost_on_extended_plan`, `base_not_cut_over_the_extended_plan`,
  `base_solid_not_made_over_the_extended_plan`, `wall_stands_in_the_added_end`)
  only where the faces of the joined walls read before the refusal show it
  short of or past them: a wall that already has Revit's end carries nothing,
  and the reason is counted in `exported.wall_end_reads_unmarked`, which also
  holds `end_join_curve_not_at_the_box_end` and `end_join_list_not_read`. The
  diagnostics count the walls that take the rule and the ends that depend on the
  order (`exported.wall_end_sources`, `wall_base_sources`,
  `wall_ends_order_dependent`); only those ends test the order, and on the office
  they are the 4 as-built ends, 3 orderings chosen after about four alternatives
  had been tried. The rule is on its own release list (Revit 2024). Only the two
  BIM4LCA office files take it (4 walls each); no wall of the other 109 exports
  of the unfiltered run does.
  On the office (Measure 42101944733 at
  `006f5f98`) the four walls have Revit's end at every height and no point of
  their plan above Revit's top; 2058499 and 2058501 at building permit go from
  0.044 ft above Revit's lowest point to Revit's, and the wall beneath them
  (1347201, a layered wall whose top a floor cut in steps) is read. **Their
  volume is not Revit's to 0.01 ft3**: it is 0.209 ft3 over (0.207 at permit)
  on all four, which is the far end's sloped top that the profile line does
  not reach (#1473), not the ends. Walls 2056705 and 2056770 at permit keep
  their ends with `joined_wall_face_inside_the_wall`: Revit cuts them back
  above 182.087 ft and the rule only adds. No other wall's body in either
  office file changes, and every other export of the unfiltered run (Measure
  42111477633 at `e5ff94f4`) is byte for byte `main`'s but Autodesk's Snowdon
  Towers Architectural, whose two refusal markers the policy above removes where
  the wall already has Revit's end. The measured effect is in
  `reports/element-framing/RE-348-wall-end-join-order.md`.

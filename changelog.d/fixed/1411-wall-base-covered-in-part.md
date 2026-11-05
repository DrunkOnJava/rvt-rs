- **A wall whose base join names a floor or footing that covers its plan only
  in part is cut at that element's top over the part it covers (#1411,
  RE-349).** Revit takes a wall's base up to the top of the element its base
  join names where the two share plan, and leaves it at the wall's own base
  where they do not. The BIM4LCA office's Basement walls 2058499 and 2058501
  as built stand past the edge of their footing (2058504, 0.24 ft of their
  plan lies outside it, 0.55 ft once their ends reach Revit's), so they kept
  their base, and RE-337 (#1398) kept their ends 0.31 ft short of Revit's
  because the longer plan would only have added base. They are now the convex
  pieces of their plan inside the footing's outline, cut along its sloped top,
  and the rest of the plan from the wall's own base (`BodyBaseSource`
  `wall_base_sloped_at_floor_covering_in_part`; `wall_base_cut_at_floor_covering_in_part`
  for a level top), a wall whose covered part is not read keeps its base and says
  so (`BaseCoverRefusal` `partly_covered_plan_not_read`). Measured on the
  office as built against Revit's export (Measure 42099826920 at `c413f382`):
  the two walls keep Revit's lowest point (174.2126 ft), take Revit's end
  (y 75.4183 ft), and their volume goes from 4.497 ft3 over Revit's (35.302
  against 30.805) to 0.147 ft3 under (30.658), a net of the upper step that
  RE-337's end then omitted and the far end's sloped top (#1473), the same as
  wall 2057670's to 3e-5 ft3. No other wall of the office as built changes (607
  bodies compared), and the office at building permit is byte for byte what
  `main` writes. An element that covers the whole plan still decides first, and
  one that covers part counts only where none covers the whole. Every other
  model is not yet measured on this head. The level top of a partial cover, a
  partially covered wall with no end extension, a footing with an opening under
  a wall and a wall whose ring is clockwise or concave are exercised by no
  measured wall. The measured effect is in
  `reports/element-framing/RE-349-wall-base-covered-in-part.md`.

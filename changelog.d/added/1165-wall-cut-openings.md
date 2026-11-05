- **The openings a fire hose cabinet and a door cut in a wall, and a curtain
  wall's where nothing trims it, are written, and every step left out is
  counted (#1165, RE-338).** A wall's native record lists the elements that
  cut it, in the order Revit's export numbers the wall's openings, and Revit's
  export writes an `IfcOpeningElement` for each that nothing fills. RE-321
  wrote the rectangular wall openings and measured these three kinds without
  writing them. A cabinet's opening is the box of the solid its symbol cuts,
  placed by its instance, a recess where it is shallower than the wall; a
  door's `WallCutoutGStep` in a wall other than its own (`m_bCutSomething` 0)
  is the door's own opening box where the wall runs across the door's wall;
  a curtain wall's is its record box, half a foot past the wall's faces,
  where the box lies half a foot inside the wall's body at both ends, no other
  wall touches it and every copy of its record agrees. On the BIM4LCA office
  as built Revit's export holds 39 cabinet, 39 door and 70 curtain-wall
  openings of these kinds and rvt-rs writes 39, 25 and 27 of them (38, 33 and
  13 of 38, 43 and 72 at permit), every one with Revit's box within 0.01 ft,
  GlobalId, name, type and both `Reference`s; none is wrong. Unfilled
  openings written go from 75 to 169 as built and from 69 to 156 at permit,
  and the census gains 178 and 198 equal property values and quantities as
  built (head 04f60dd6, Measure run 41952363750). A door's opening 10 ft
  across a wall that runs along its own wall, and a curtain wall that reaches
  its wall's end, has another wall beside it or has record copies that
  disagree, are not written: `exported.wall_cut_openings_left_out` counts each
  step left out by reason, and the counts close. Three openings as built and
  four at permit (one of them RE-321's) are written that Revit's export of
  the same file lacks and its export of the sibling file holds, and the
  scorer counts them apart; their records are not byte for byte the sibling
  file's, which RE-338 section 6 gives. The thresholds were fitted on the
  office pair alone: Core Interior and RE1 Architecture are unchanged, the 19
  held-out and spent models' exports are byte for byte `main`'s (Measure run
  41954664033) and the Autodesk samples' are but `rac_basic` 2024 and 2025,
  which each gain one opening.

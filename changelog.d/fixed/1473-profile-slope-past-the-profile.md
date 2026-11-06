- **A wall whose elevation profile was edited keeps its cut-out past the
  profile, to the wall's end (#1473, RE-358).** The sketch of a profile spans
  the wall's location curve, but the wall's body goes on past it where a join
  extends the wall, and the opening that voids the wall (RE-151) stopped at the
  profile's end: the stretch stayed as the wall's box had it while Revit's body
  goes on along the profile. Where a wall goes on past an end of its profile at
  a joined end and an opening reaches that end with one vertical edge, the
  opening is carried on to the wall's end in two shapes: a sloped line along its
  own slope, and the cut-out of a notch (both neighbours level) straight out,
  told apart from the wall's own top or base by height and not by the order the
  outline lists them in (`ProfileStubSource` `wall_profile_sloped_past_*`,
  `wall_profile_level_past_*` or `wall_profile_sloped_and_level_past_*`). A wall
  that does not read so keeps what it had, with `ProfileStubRefusal`. Read on
  Revit 2024 only, on its own release list, and counted in
  `exported.wall_profile_sources` and `exported.wall_profile_refusals`.
  Measured on the BIM4LCA office: eight Basement walls take it on each file (six
  high ends, two low ends), each Revit's volume to 1e-4 ft3 (the four of #1473
  were 0.209 ft3 over, 0.207 at permit) with no top above Revit's, and two of
  them (#1474) move 0.2 ft3 towards Revit's. Two walls of Autodesk's `rac_basic`
  and one of Snowdon Towers Architectural take it as well; no Revit export
  checks the `rac_basic` walls (the report is
  `reports/element-framing/RE-358-profile-slope-past-the-profile.md`).

- **A wall whose elevation profile was edited keeps sloping its top past the
  profile, to the wall's end (#1473, RE-358).** The sketch of a profile spans
  the wall's location curve, but the wall's body goes on past it where a join
  extends the wall, and the opening that voids the wall (RE-151) stopped at the
  profile's end: the stretch stayed flat at the top of the wall's box while
  Revit's top goes on along the profile's own line. The opening's line is now
  carried on to the wall's end along its slope, where the wall goes on past the
  profile and the opening reaches that end with one vertical edge (a sloped
  line's end and a rectangle corner); a wall that does not read so keeps the top
  it had, with `ProfileStubRefusal`. Read on Revit 2024 with the profile's
  openings, counted in `exported.wall_profile_sources` and
  `exported.wall_profile_refusals`. Not yet measured; the report is
  `reports/element-framing/RE-358-profile-slope-past-the-profile.md`.

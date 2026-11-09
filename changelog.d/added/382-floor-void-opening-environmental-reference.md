- **The opening of a host's own geometry carries the host's type name as
  `Pset_EnvironmentalImpactIndicators.Reference` too, as Revit's export writes
  it (RE-382).** A floor's sketch void and the opening a face-hosted door or
  window cuts in its host (RE-237) had the host's `Reference` in
  `Pset_OpeningElementCommon` alone, where Revit's export holds it in both sets
  on every opening. It does so on 37 of the BIM4LCA office's openings as built
  and 36 at building permit (`VP-1` on its floors' voids, `YP-1` on its roof's
  hatch) and on 12 of Snowdon Towers' (local only), each equal to the host's.
  Core Interior's export writes no property sets at all (its setup), so it shows
  nothing, and rvt-rs writes the set there as it does for every other element
  of Core.

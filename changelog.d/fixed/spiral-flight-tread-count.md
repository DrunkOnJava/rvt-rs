- **A spiral stair flight's `Pset_StairFlightCommon.NumberOfTreads` is
  written (RE-287).** The BIM4LCA office's assembled stair flight 3057703 was
  the one flight of 35 whose tread count rvt-rs left out, where Revit's export
  writes 18: a spiral run's plan is an arc, so its begin- and end-with-riser
  flags are not read and its count was not computed. A spiral run whose treads
  are drawn as its body now gives its flight the number of those treads, and
  with `--revit-set-placement` its stair's flight set sums it; no other value
  on that file changes.

- **Level beams' `Pset_BeamCommon.Slope` (W7).** For a beam it exports as an
  extrusion, Revit's export takes the slope from a parameter named `Slope`,
  then from the extrusion's own slope, and otherwise as the angle between the
  beam's location line and the horizontal, 0 for a line in the plan. rvt-rs
  writes 0 on the beams it writes an extrusion length for whose location line
  stays in the plan: on the BIM4LCA office, 8 of 8 beams as built and at
  building permit, each Revit's value. A sloped beam gets none, as the
  extrusion's slope it would take is not measured.

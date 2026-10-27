- **What a wall's `Height` is (RE-257, #1016).** Revit's export writes the
  vertical span of its own body: rvt-rs's wall body is that tall on 501 of
  the BIM4LCA office's 579 walls as built and all 117 of puzzbobb's R2025
  walls, but taller on the office's other 78, which Revit draws cut (#1067).
  Neither the wall's saved Unconnected Height nor any field of its record
  tells those apart, so no wall gets a `Height` yet.

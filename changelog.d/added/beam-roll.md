- **Unrotated beams' `Pset_BeamCommon.Roll` (W7, RE-276).** Revit's export
  writes a beam's Cross-Section Rotation, in radians, as its `Roll`. A
  beam's native record holds it in the `m_rotation` of its
  `FamInstCurveDriver`. rvt-rs reads it and writes `Roll` 0 for the beams it
  writes an extrusion length for whose rotation is exactly 0: on the BIM4LCA
  office, 8 of 8 beams as built and at building permit, each Revit's value. A
  rotated beam gets none, as no measured beam is rotated.

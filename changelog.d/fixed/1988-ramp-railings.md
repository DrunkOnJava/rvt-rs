- **A railing that names a ramp and has no level of its own is aggregated under
  the ramp on Revit 2024, as Revit's export aggregates it (#1988).** Revit's
  export writes the BIM4LCA housing's ramp as an `IfcRamp` with its two
  railings as parts, where rvt-rs wrote them as plain elements of their
  storey. The office's eight railings that name a ramp each have a level of
  their own, and Revit's export leaves them plain, so they stay plain.

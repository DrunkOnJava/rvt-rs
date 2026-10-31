- **Beams' `Pset_BeamCommon.Span` (W7).** Where it draws a beam as an
  extrusion and no parameter gives the span, Revit's export writes the
  extrusion's length as the beam's `Span`, the length its
  `Qto_BeamBaseQuantities.Length` holds. rvt-rs writes it for the beams it
  writes that length for, those drawn along their location line (RE-49): on
  the BIM4LCA office, 8 of 8 beams as built and at building permit, each
  Revit's value.

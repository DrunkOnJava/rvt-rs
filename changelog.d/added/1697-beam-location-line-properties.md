- **A beam gets the `Length`, `Span`, `Slope` and `Roll` of its location line, whatever its body is drawn from (#1697, RE-428).**
  Revit's export writes `Qto_BeamBaseQuantities.Length` and `Pset_BeamCommon.Span` as the
  length of the beam's `Axis` curve, `Slope` as that curve's angle to the horizontal and
  `Roll` as the Cross-Section Rotation, and rvt-rs wrote them only where it had solved the
  beam's body along that line (`Slope` only for a level one). The line of the beam's
  element data that lies inside its record box now gives all four wherever the body comes
  from: a box along the line, a box longer than the line (RE-328, which held that no Revit
  export measures a beam longer than its line; Revit writes the line's length on all 39 of
  them), a box no section along the line closes, or a mesh. They read equal to Revit's on
  all 291 beams of the BIM4LCA housing and offices that have a line (36 of them sloped),
  none different, with 246 values added in the housing's four files. A beam with no line (9
  in those files), a beam cut by a column with no line, and a `Roll` whose rotation is not
  read as 0 stay without, as before. Across the housing and 29 Autodesk samples, which no
  Revit export scores, 700 beams gain properties and no element is added or removed; the
  beams' bodies do not change. The line is the one the beam's body already reads, so the
  change adds no pass. Measured on one project with a Revit export (the BIM4LCA office and
  housing, Revit 2024); the IFC2X3 oracle models, whose Revit exports hold no `Span` or
  `Slope` for `rac_advanced` 2015 to 2020's 8 beams, are counted apart.

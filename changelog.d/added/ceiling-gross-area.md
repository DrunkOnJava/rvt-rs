- **Ceilings' `GrossArea` (#1016, RE-297).** Revit's export writes a
  compound ceiling's `Qto_CoveringBaseQuantities.GrossArea` as the area of
  the profile its sketch gives, less the cut where Revit cuts it, and a cut
  shows in the ceiling's native record as a `CutPairGStep`. A ceiling drawn
  as one extrusion of its sketch whose record holds none carries that area,
  its holes taken off: 57 of the 90 values Revit's export writes on the
  BIM4LCA office as built and 45 of 75 at building permit, none different.
  The area of a ceiling in pieces (Revit's export gives one piece's), of a
  cut one and of the office's roofing coverings, ventilation units drawn as
  boxes, is not written.

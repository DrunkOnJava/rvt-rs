- **A tube beam's six base quantities are not its saved mesh's (#1698,
  RE-442).** On the BIM4LCA office as built, the saved mesh of each of the 8
  hollow tube beams gives `CrossSectionArea`, `OuterSurfaceArea`,
  `GrossSurfaceArea`, `NetSurfaceArea`, `GrossVolume` and `NetVolume` within a
  relative 1.1e-4 to 4.4e-4 of Revit's export, short every time by the chords
  of its rounded corners, where the census reads equal within 1e-5: all 48
  values would read different, so none is written. The profile Revit's export
  draws (two rounded rectangles) and the length give all six within the
  tolerance, so they arrive with the tube's body. A probe,
  `probe_beam_tube_quantities`, prints what each beam's mesh gives.

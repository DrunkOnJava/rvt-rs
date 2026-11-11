- **A column drawn as its type's hollow section gets the `NetVolume` Revit's
  export writes (RE-247).** `Qto_ColumnBaseQuantities` now holds it beside the
  `CrossSectionArea`, `OuterSurfaceArea` and `GrossVolume` such a column already
  holds: not the section's area times the length but the volume Revit computes
  for it, the section's outside rectangle less its inside one times the
  extrusion's depth, which the arcs of its corners are missing from. Revit
  writes that value exactly on the five columns of the BIM4LCA office's three
  hollow-section types checked (PI-2, PI-4 and PI-5, at five lengths, to the
  last digit it writes), and the office as built is the file where rvt-rs leaves
  it out, on 79 hollow-section columns, its 80th being the round column, which
  stays without. Measured on one project with a Revit export (the BIM4LCA
  office, Revit 2024).

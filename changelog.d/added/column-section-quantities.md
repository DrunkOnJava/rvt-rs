- **A hollow-section column's area, surface and volume (#1016, RE-247).** A
  column joined to nothing and drawn as its type's hollow section carries
  `Qto_ColumnBaseQuantities.CrossSectionArea`, `OuterSurfaceArea` and
  `GrossVolume` from that section, its rounded corners counted as arcs:
  Revit's values on all 79 such columns of the BIM4LCA office as built and
  55 at permit, none different. Its `NetVolume` is left out, as Revit's
  export writes the smaller volume Revit computes, and a column drawn as its
  record's box gets none of the three, as a round column is drawn as one too.

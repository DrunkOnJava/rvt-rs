- **A curtain panel drawn from its saved mesh as a vertical box carries its
  `Width`, as Revit's export writes it (RE-396).** Revit's export gives a curtain
  panel `Qto_PlateBaseQuantities.Width` the longer of its horizontal sides, and
  gives none to a panel whose body is not a box. Where rvt-rs draws a panel from
  the mesh Revit saved for it (the default since #1191) and that mesh is a
  closed box of flat faces with its smallest side horizontal, the longer of the
  two horizontal extents is the quantity: Revit's on all 257 panels of the
  BIM4LCA office as built (266 at building permit), none of which had one, and
  the 3 panels whose mesh is not a box, which Revit gives none, get none.

- **A beam gets the six base quantities Revit's export writes for it (#1693, RE-423).**
  `Qto_BeamBaseQuantities` now holds `CrossSectionArea`, `OuterSurfaceArea`,
  `GrossSurfaceArea`, `NetSurfaceArea`, `GrossVolume` and `NetVolume`, read from the
  mesh Revit saved for the beam where that is one closed solid of flat faces with no
  hole through it, a prism along its longest axis with two flat, square ends of one
  area, and its volume is the end area times its extent: the sums of the prism
  Revit's export extrudes the beam's profile into (the end's area, the lateral area,
  the lateral area and both ends, the volume). The beam's body does not change. They
  read equal to Revit's on 21 of the BIM4LCA concrete housing's 22 beams, 41 of the
  timber housing's 41 and 71 of the timber office's 71, none different; the concrete
  office's 8 hollow tubes, whose Revit values are no sum of their faces, get none.
  Autodesk's `rac_advanced`, `rst_basic` and Snowdon Towers Structural gain them on
  938 beams, which no Revit export scores. Measured on one project with a Revit
  export (the BIM4LCA office and housing, Revit 2024).

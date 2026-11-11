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
  export (the BIM4LCA office and housing, Revit 2024). Reading the beams' saved
  graphics costs time and memory on some files (Measure run 42972934548, base and
  head timed in turn on one runner, three rounds): Autodesk's `rst_basic` of 2023
  to 2027 take 21 to 29% longer and peak at 2.1 to 2.3 times the memory (143 to
  147 MiB become 314 to 327 MiB), `rac_advanced` of 2023 to 2027 take 14 to 16%
  longer and peak at 1.8 to 1.9 times (557 to 579 MiB become 1,010 to 1,090 MiB),
  and Snowdon Towers Structural of 2024 takes 69% longer and peaks at 2.5 times
  (494 MiB becomes 1,224 MiB). The other 57 of the 68 files timed read 0.99 to 1.01
  in time and 0.96 to 1.00 in peak memory; the BIM4LCA housing and timber office
  are not among the 68. `--no-saved-meshes` runs no pass, so it leaves out these
  quantities and this cost.

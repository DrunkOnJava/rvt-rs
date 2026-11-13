- **A roof by extrusion is a solid on Revit 2024, 2026 and 2027 as on 2025,
  held to the same project's 2025 save (#1479, #1864, RE-458).** The release
  list of the B-spline reader held Revit 2025 alone, so such a roof was its
  saved mesh on 2024, 2026 and 2027 (its record box before #1191). Autodesk's
  `rac_basic` has one, roof 243274: its 2024, 2026 and 2027 saves now draw it
  as the 2025 save does (35.424 m3, where its mesh was 34.974 m3 on 2026 and
  2027), and are the only 3 of 146 exports of an
  unfiltered Measure that change; every body of the project's 2026 and 2027
  saves is the 2025 save's. No Revit export of such a roof of those releases
  exists here, so the check is the 2025 save. `rac_advanced`'s roof 201262,
  which #1479 took for a second one, is not a roof by extrusion and is
  unchanged.

- **Family instances, mullions, columns and walls carry Revit's materials (#355).**
  Read from the native records instead of byte offsets and scans
  (RE-201 to RE-206):
  - a mullion or curtain panel takes its type's `m_renderStyleId`;
    RE-166's fixed offset moved on 10 of Snowdon Towers' 13 mullion types
    (RE-201);
  - an off-centre column takes the layer of the wall it was first joined
    to on its side, or the exterior layer where it stands out past both
    faces; rendered content (RPC, planting) is `<Unnamed>` (RE-202,
    RE-205);
  - a type's materials are its `m_geomTag2MaterialId` read by name, not
    a nested component's; a material's name is its own `Material.m_name`
    (RE-203);
  - `<Unnamed>` only where the family's own document shows geometry with
    no material, and a material only hidden forms have, nested families'
    included, is left out (RE-204, RE-205, RE-206);
  - a layer that takes its category's material takes the one the
    category's graphics styles name where the object styles are unset or
    not read, as on every Revit 2025 file (RE-206).
  On the R2025 models every wall now has Revit's "Default Wall", 117 in
  21 models, none before. RE1 Architecture 71 of 71 and RE1 Electrical 49
  of 49 elements have Revit's set (70 and 47 before). Locally on Snowdon
  Towers 5,134 of the 5,134 elements rvt-rs gives materials have Revit's
  set (3,689 after RE-200). Core Interior exports as before. The material
  names and category styles are memoised, and Snowdon's export takes
  23 s against 20 s before. By puzzbobb (#886).

- **A family instance takes the materials its own parameters name (#355).**
  Where a family type's geometry-material map holds no material, #580
  gave its columns and products `<Unnamed>`. Where the instance's own
  material parameters (`FamilyInstance.m_pInstParams`) name materials,
  Revit's export gives it those instead, and now so does rvt-rs (RE-200).
  On the R2025 models 12 to 17, the 9 window trims and muntins of each now
  have Revit's material set: all 13 elements rvt-rs gives materials have
  Revit's set, 4 before.
  Locally on Snowdon Towers, 232 more elements have Revit's set and none
  fewer. Core Interior, the RE1 models and the other R2025 models export
  as before. By puzzbobb (#886).

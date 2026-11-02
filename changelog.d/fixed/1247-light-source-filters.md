- **Saved meshes leave a light fixture's light source out of its body,
  instead of keeping the fixture's box.** The light source is saved as a
  filter of its own: a `GImposter` (a rendering asset with no surface)
  with the shape and lines that draw the light, none of it in Revit's
  export. On the BIM4LCA office 39 more bodies are drawn from their saved
  meshes, all exact to 0.001 ft against Revit's export, as are RE1
  Electrical's 2. Snowdon Towers' electrical sample draws 1,279 of its
  1,465 bodies from saved meshes instead of 907 (#1247, RE-300).

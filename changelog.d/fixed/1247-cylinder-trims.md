- **Saved meshes read cylinder faces whose trim is not a rectangle in their
  UV box.** Such a face was rejected, and with it the whole element, which
  kept its box. The face is now meshed in strips across its angle, each no
  wider than the chord bound. On the BIM4LCA office 155 more doors are drawn
  from their saved meshes, as the default export draws doors: 151 exact to
  0.001 ft against Revit's export, and all within 0.01 ft and 1% of its
  volume (#1247, RE-304).

- **A plane face whose outline is one ring that visits a vertex twice (two lobes touching at a point) is split there and meshed ring by ring when its triangles miss its area, and 21 sanitary terminals of Autodesk's Snowdon Towers Architectural are drawn from their saved meshes in place of a box (#1732, RE-434).**
  The 21 are one toilet type, `Toilet-Domestic-3D`, whose symbol holds a plane
  face with a ring of 22 points that visits its crown twice (two thin spandrels
  under a flat arch, beside two corner pieces): earcut, given the ring whole,
  covered a sliver under the crown too, its triangles added up to 0.65% more
  than the ring's area, and the face was refused for `triangulated area
  mismatch` once RE-430 let the toilets reach it. The rings are split at a
  repeated vertex only after that check fails, so no element that is drawn
  changes: of 126 exports of the unfiltered run 125 are byte for byte the
  previous build's, and Snowdon Towers Architectural moves by exactly these 21
  elements (4,282 drawn on both sides, 0 of their meshes differ; the refusals
  `triangulated area mismatch` 21 to 0, `saved_graphics_mesh` 4,282 to 4,303).
  The 21 read 1.0063 of Revit's volume (21 of 21 within 1%), and Revit's
  vertices lie up to 0.0047 ft from the mesh, on both sides of it, as for the
  toilets of the BIM4LCA office. The model's IFC grows from 269.5 to 296.3 MB
  (+9.9%): each toilet carries 34,260 triangles where Revit's has 1,668
  (#1737). Export time and peak memory read 1.02 and 1.03 of the previous
  build's on that model.

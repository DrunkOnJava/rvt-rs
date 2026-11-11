- **A plane that meets a cylinder along an edge running across both of the
  cylinder's parameters is meshed, and 175 elements of Autodesk's Revit 2024
  samples are drawn from their saved meshes in place of a box (#1706, RE-426).**
  Snowdon Towers Architectural had 145 elements refused with `shared curved
  edge is not an isoparametric interval` or `shared curved edge changes its
  fixed parameter` (a slanted plane through a cylinder, a mitred rail): 126
  are drawn now and 19 stay boxes for the next reason. `rac_advanced` gains
  21, `rac_basic` 3, `rme_advanced` and `rme_basic` 8 each, and `rst_basic`
  9 beams. The edge takes, on the plane's polygon and on the cylinder's loop
  alike, the points of the edge itself that keep its chords within 0.00025 ft
  of the edge. The 102 of the 126 that are not railings are read against
  Revit's IFC4 export of the model (local only) and agree: 100 building
  element proxies, a slab and a roof within 1% of Revit's volume, with every
  vertex of each body within 0.001 ft of the other's surface on 101 of them
  and within 0.01 ft on the other. The 24 railings read about 1.2 times
  Revit's volume (two are within 1%), as the 101 railings drawn before this
  change do (34 within 1%): Revit's export draws their round bars as polygons
  with fewer sides inscribed in the circle the mesh draws, and the scorer
  reads 1,792 of the 1,937 shells of 22 of them as exactly that; the 145 it
  does not read are #1712. The 49 elements of the other samples are not
  scored, no Revit export of them existing. No element that was drawn
  changes (0 of 19,207). The same change draws 25 railings in each of the
  four BIM4LCA offices (24 of the 25 within 1% of Revit's volume), 1 and 2
  in the terrain files, 2 in each of four held-out files and the same
  elements in the other releases of the samples (38 of 126 exports of the
  unfiltered run differ). An export that draws more is larger: Snowdon
  Towers Architectural 196.6 to 229.7 MB (+16.8%), `rac_advanced` +18% to
  +20%, the offices +9% to +11%, the rest 1% to 4%; export time stays within
  4% and peak memory within 3% of the previous build's on the same runner.

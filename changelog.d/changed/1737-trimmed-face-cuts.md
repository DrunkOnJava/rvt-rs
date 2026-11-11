- **A face cut by its trim is cut across at the chord's `v` steps alone, where every trim point's `v` was a cut as well, and the triangles of 25 of 126 exports fall (#1737, RE-437).**
  `trimmed_cells` cut every strip's trapezoids at each knot of `v`, and
  `face_grid` made every trim point's `v` a knot, so a trim of T points left
  every trapezoid that held a point's `v` cut once more across the whole
  face. The strips keep a knot at every trim point's `u` (a trim segment must
  cross a strip whole, so each corner of the trim is still a corner of the
  cells) and the cuts across them are the chord's steps; a face meshed on its
  grid keeps every trim point as a knot, and a plane face that shares an edge
  with a cut face follows the same `v` knots. 25 of the 126 exports of the
  unfiltered run move, by the meshes of 766 of the 82,295 objects drawn from a
  saved mesh on both sides of the 24 whose IFC was kept (470 furniture, 252
  sanitary terminals, 36 coverings, 5 railings, 2 beams and a space), and no
  element drawn before stops being drawn. The triangles from saved graphics
  fall by 8.1% on the BIM4LCA office (a toilet from 14,025 to 11,314), 8.7% on
  Snowdon Towers Architectural, 5.5% on the large housing exports and 15% on
  `rac_advanced`, and the IFCs by 0.5% to 9.4% (Snowdon Towers Architectural
  272.1 to 253.8 MB). Against Revit's export no class's median volume moves
  and no count of bodies within 1% of its volume, or with every vertex of
  Revit's within 0.001 ft of the mesh, falls; one body of RE1 Architecture, a
  shower head, has its farthest vertex from Revit's surface move from 0.0091
  to 0.0100 ft. Export time reads 0.95 to 1.03 and peak memory 0.96 to 1.04
  of the previous build's.

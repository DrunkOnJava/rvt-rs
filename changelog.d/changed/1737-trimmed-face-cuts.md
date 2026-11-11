- **A face cut by its trim is cut across at the chord's `v` steps and at the `v` of the trim's own steps and cusps, where every trim point's `v` was a cut, and the triangles of 25 of 126 exports fall (#1737, RE-437).**
  `trimmed_cells` cut every strip's trapezoids at each knot of `v`, and
  `face_grid` made every trim point's `v` a knot, so a trim of T points left
  every trapezoid that held a point's `v` cut once more across the whole
  face. The strips keep a knot at every trim point's `u` (a trim segment must
  cross a strip whole, so each corner of the trim is still a corner of the
  cells) and the cuts across them are the chord's steps and the `v` of the
  vertices the trim does not run through from one strip to the next (the ends of
  a step along a strip boundary and the tip of a hole): without those a strip's
  cells run straight past a vertex of the next strip's and leave a crack inside
  the face, which a first version of this change did on all 43 toilets of the
  BIM4LCA office. A face meshed on its grid keeps every trim point as a knot,
  and a plane face that shares an edge with a cut face follows the same `v`
  knots. 25 of the 126 exports of the unfiltered run move (the 43 toilets of each
  BIM4LCA office among them), and no element drawn before stops being drawn. The triangles from saved graphics fall
  by 7.5% on the BIM4LCA office (a toilet from 14,025 to 11,516), 6.8% on Snowdon
  Towers Architectural, 4.6% on the large housing exports and 10.4% on
  `rac_advanced`, and the IFCs by 0.4% to 6.6% (the office 117.1 to 113.6 MB,
  Snowdon Towers Architectural 272.1 to 257.6 MB). The vertices of a face's
  cells that lie inside a boundary edge of the same face's cells are as many as
  before, class by class, on the office, the timber office, the housing and RE1,
  none in a toilet. Against Revit's export no class's median volume moves and
  no count of bodies within 1% of its volume, or with every vertex of Revit's
  within 0.001 ft of the mesh, falls; one body of RE1 Architecture, a shower
  head, has its farthest vertex from Revit's surface move from 0.009123 to
  0.01007 ft, over the 0.01 ft column. Export time reads 0.97 to 1.02 and peak
  memory 0.98 to 1.05 of the previous build's.

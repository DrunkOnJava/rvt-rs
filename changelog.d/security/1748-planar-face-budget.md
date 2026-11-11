- **A planar face whose rings hold more than 10,000 points, or whose boundary
  search and ring split would take more than 250 million steps between them,
  is refused instead of triangulated (#1748).** Earcut, the search that puts
  back the boundary points earcut leaves out, and the search for a ring that
  visits a vertex twice grow faster than the points of a face, and a file can
  give each edge of a ring up to 100,000 of them, so a hostile ring could take
  an export hours. The steps are counted for the face in all (its outer rings
  and the second try after a split share the budget). The largest planar face
  of the 2023 to 2027 samples, the BIM4LCA models and the reference models
  holds 2,308 points and the most one call of the boundary search asks is 14.6
  million steps, so no export of the corpus changes; a face over either budget
  keeps its box, as every other refused face does, and says `planar face
  budget exceeded`.

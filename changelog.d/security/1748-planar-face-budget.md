- **A planar face whose rings hold more than 10,000 points, or whose boundary
  search would take more than 250 million steps, is refused instead of
  triangulated (#1748).** Earcut, the search that puts back the boundary points
  earcut leaves out, and the search for a ring that visits a vertex twice grow
  faster than the points of a face, and a file can give each edge of a ring up to
  100,000 of them, so a hostile ring could take an export hours. The largest
  planar face of the 2023 to 2027 samples, the BIM4LCA models and the reference
  models holds 2,308 points and asks 14.6 million steps, so no export of the
  corpus changes; a face over either budget keeps its box, as every other
  refused face does, and says `planar face budget exceeded`.

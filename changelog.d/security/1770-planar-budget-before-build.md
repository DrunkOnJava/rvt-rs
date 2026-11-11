- **A planar face is refused for its points as each edge of its rings is read,
  and the tests of a face with regions count against its work, instead of after
  the ring is built (#1770).** #1748 refused a face of more than 10,000 points
  or 250 million steps, but counted the points once a ring was built, after
  each of its edges had been refined up to 100,000 points, and charged nothing
  for a face with regions, whose rings were earcut with no cap and whose every
  region vertex was tested against every parent vertex. The points an edge may
  still add are now what is left of the face's budget when the edge is read:
  the graph's own points are counted before any is copied, and the edge's
  refinement stops with `planar face budget exceeded` at the last point the face
  may hold; the parent's ring and the regions' rings share one such budget, and
  the vertex tests of a region (its vertices times the parent's) are charged to
  the face's work. The budgets' values are as #1748 chose them (10,000 points,
  250 million steps; the largest planar face of the corpus holds 2,308 points),
  so no export of the corpus changes; a face over either keeps its box, as every
  other refused face does.

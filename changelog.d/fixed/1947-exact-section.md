- **A solid the export writes from a plan is never its plan's bounding box, nor
  loses its holes (#1947).** A sloped slab, a roof drawn along its planes and a
  wall's base cut along a footing's top were meshed with the viewer's drawing
  rule: a plan that did not triangulate became its bounding rectangle, and a
  hole that could not be joined was filled. Such an element now keeps the body
  its export falls back to, as it does for any other plan it cannot draw. The
  viewer and the plan SVG still draw the rectangle. No measured model
  holds such a plan: 61 of 61 exports are unchanged.

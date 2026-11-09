- **A room whose stored solid neither the walk nor the span closes takes its
  floor's outline from the solid's edges, on Revit 2024 (#90, RE-374).** A
  room solid lists its faces and, before them, its edges. Each edge names
  its two faces and gives each face's (u, v) at two of its points, so a wall
  face that is not a rectangle gives its real end, which its uv box (all the
  walk and the span read) does not. The floor's edges, each drawn as the
  wall it meets, give the outline (`SpaceOutlineSource` `stored_solid_edges`,
  counted in `exported.edge_outline_spaces`).

  The BIM4LCA office's room 3095831 at building permit, which kept its box,
  is now 1864.737 ft² against the 1864.74 of Revit's IFC4 export, and
  Snowdon Towers' room 830902 304.059 ft² against Revit's 304.058 (the
  model's later VIM edition, local only). On every room of Autodesk's samples,
  the office and the Revit 2025 models that the walk or the span already
  reads and this rule accepts, the edges give the same outline, to 0.01 ft²
  in area and 0.001 ft on its extent.

  It is read only where every edge's two planes give it the same ends and
  the solid holds no point between its floor and 4 ft above it except on a
  vertical edge, so that a raised floor, a step, a pocket or a low bulkhead
  refuses it. Snowdon Towers' room 2181757 shows why: its floor is 5172.857
  ft², its section 4 ft above the floor is 5177.493, and Revit's area in the
  model's later VIM edition is 5177.492. It keeps its box.

- **A ruled, revolved or cone face whose trim is not the rectangle of its parameters, or has holes, is meshed in the cells of its grid, and 119 elements of Autodesk's Snowdon Towers Architectural, 43 of each BIM4LCA office and 71 of each `rac_advanced` save are drawn from their saved meshes in place of a box (#1706, RE-430).**
  A face whose one loop did not run round the rectangle of its parameters (the
  outline of a revolved bowl, a hole in a plate, two loops of one face) was
  refused as `nonrectangular parametric trim` or `multiple parametric trim
  loops unsupported`. The `u` of every point of every loop is already a knot
  of the face's grid, so the loops cut the region they enclose into strips,
  cut across at the grid's own `v` steps (#1737), each cell is fanned into
  triangles, and a triangle without area whose edges
  are all 1e-4 ft or shorter, or whose shortest edge is 1e-8 ft or shorter, is
  dropped as a pole's is; one without area and a longer edge still refuses the
  face. Furniture, sanitary terminals, coverings, one railing, two beams and a
  space change from `partition_element_record_bbox` or
  `partition_family_instance_turned_box` to `saved_graphics_mesh`: Snowdon
  Towers Architectural 119 (102 chairs, 15 sinks, a faucet, a space), the
  offices' 43 toilets each, the large housing exports 15 each and the terrain
  ones 9, `rac_advanced` 71 per save (2023 to 2027), `rac_basic` 1, Snowdon
  Towers Plumbing 13 and Structural 2, RE1 Architecture 1. 25 of 126 exports
  of the unfiltered run move, and no element that was drawn changes (0 of the
  77,842 drawn on both sides of the 24 whose IFC was kept). The 176 new
  elements that Revit's export holds are scored against it: 73 have Revit's
  volume within 1% (the offices' toilets at a median 1.0060, the housing's
  and Snowdon's sinks), the 102 chairs read 1.0164 (Revit draws their round
  tubes as inscribed polygons) and the faucet 1.0992, and no body has every
  vertex of Revit's within 0.001 ft of rvt-rs's surface: Revit's own
  vertices lie up to 0.0043 ft (the chairs) and 0.0050 ft (a toilet) from it,
  on both sides. An export that draws more is larger, and by the triangles:
  an element drawn from a revolved or ruled face carries 4,600 to 17,500 on
  average (Revit's toilet has 1,668), so Snowdon Towers Architectural grows
  from 229.7 to 269.4 MB (+17.3%), the offices by 27.8% to 31.2%, the large
  housing exports by 27% to 29% and the terrain ones by 61%, `rac_advanced` by
  53% to 56% and `rac_basic` by 8.6%. Export time reads 0.98 to 1.03 and peak
  memory 0.95 to 1.02 of the previous build's on every file of the timing run
  but the `rac_advanced` saves, which read up to 1.09 and 1.06: 71 elements
  add 759,222 triangles to a pass of six to eight seconds. 21 sanitary
  terminals of Snowdon Towers Architectural whose plane faces have holes keep
  their boxes (#1732), and a cylinder face with several loops is refused as
  before.

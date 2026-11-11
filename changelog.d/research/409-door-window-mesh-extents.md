- **A door's saved mesh, tried for its `OverallHeight` (#1636, RE-409).** Where
  no cut loop gives a door's size, the height of its saved mesh is Revit's
  `OverallHeight` on the 65 doors of the BIM4LCA timber office at permit whose
  written height is 15 or 45 mm over it, and a rule that takes it makes those 65
  equal. It also turns the one door of RE1 Architecture's export that it reaches,
  equal before, 1.9 mm short, and the housing exercises it on no door, so nothing
  ships. `tools/re/door_window_mesh_extents_vs_ifc.py` reads, for each door and
  window, the size written and the mesh's extents against Revit's.

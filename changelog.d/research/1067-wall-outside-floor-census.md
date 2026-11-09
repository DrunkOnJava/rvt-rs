- **Revit keeps a wall's own height over the part of its plan outside the
  floors that cut it on 191 of 225 pieces of five buildings, and cuts it on
  5; nothing that rests on more than one wall separates them (#1067,
  RE-375).** `tools/re/wall_outside_floor_vs_ifc.py` reads Revit's own bodies
  for those pieces. The stub of the BIM4LCA office's wall 3004978 as built,
  which RE-320 leaves at its full height, is the one piece of them under
  0.05 ft² and inside other walls: Revit's roof leaves the same stub
  uncovered, the wall's Axis starts at the roof's edge, and Revit cuts the
  stub with the rest of the wall. Three readings fit it, and each holds one
  wall.

- **A wall of several layers gets no `GrossFootprintArea` (RE-406, #1636).**
  Revit's export writes it on a wall drawn as one extrusion and on none of the
  walls of a type with several layers, which it draws layer by layer. On the
  BIM4LCA housing, concrete as built, 36 such walls carried a footprint area
  that Revit's export does not hold.

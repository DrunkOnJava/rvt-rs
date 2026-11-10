- **A wall's box gives its quantities and its `Height` only where Revit's solid
  is that box (RE-405, #1636).** A wall whose saved mesh is whole and has
  another volume than its box (a wall with an angled end, a box longer than the
  wall) loses the quantities taken from the box and takes the mesh's
  `NetVolume` and, of a wall of no more than four vertical faces, its areas. A
  wall whose type has several layers is given none of its box's quantities and
  no `GrossFootprintArea`, which Revit's export omits on it, and takes the
  mesh's where that is whole. Its `Height` is written where the mesh's vertical
  span confirms the extrusion's height, and dropped where it does not, as on a
  wall whose lower part is cut away. On the BIM4LCA housing every wall `Height`
  that differed from Revit's is gone and several hundred are written.

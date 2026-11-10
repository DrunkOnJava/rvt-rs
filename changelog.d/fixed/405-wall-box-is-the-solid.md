- **A wall's box gives its quantities and its `Height` only where Revit's solid
  is that box (RE-405, #1636).** A wall whose saved mesh is whole and has
  another volume than its box (a wall with an angled end, a layered wall whose
  layers Revit trims, a box longer than the wall) loses the quantities taken
  from the box and takes the mesh's `NetVolume` and, of a wall of no more than
  four vertical faces, its areas. Its `Height` is written where the mesh's
  vertical span confirms the extrusion's height, and dropped where it does not,
  as on a wall whose lower part is cut away.

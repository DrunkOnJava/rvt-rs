- **A wall's quantities from its saved mesh set aside each opening at its
  real height.** Where a door's or window's opening starts above its
  placement (RE-93), the box `wall_mesh_quantities` sets aside for it was as
  tall as its start plus its top, so the faces of the wall above a window up
  to that height were taken for the opening's. The box is now the opening the
  writer cuts: from its start, as high as it is. Since #1741; found by the
  batch 9 export audit.

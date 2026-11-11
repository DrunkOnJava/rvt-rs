- **A wall whose saved mesh is a vertical prism less the openings it carries
  gets its `NetSideArea`, `GrossSideArea`, `GrossVolume` and `Height` where its
  base is not attached (#1046).** Every horizontal face of such a mesh between
  its lowest and highest point must be the top or the bottom of an opening
  rvt-rs cuts in the wall, and the two largest vertical faces must still agree
  on the gross area; Revit's export writes the four on all 170 such walls of
  the BIM4LCA office's two exports whose base is not attached and on none of
  the 5 whose is. The rule is measured on Revit's own bodies of one project in
  two states, not yet on the saved mesh rvt-rs reads nor on other projects.

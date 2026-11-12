- **A wall whose saved mesh is a vertical prism less the openings it carries
  gets its `NetSideArea`, `GrossSideArea`, `GrossVolume` and `Height` where its
  base is not attached (#1046).** Every horizontal face of such a mesh between
  its lowest and highest point must be the top or the bottom of an opening
  rvt-rs cuts in the wall, at least one opening must be cut in it, every
  vertical face beyond the four of a prism must be a jamb of one of those
  openings, and the two largest vertical faces must still agree on the gross
  area; Revit's export writes the four on all 170 such walls of the BIM4LCA
  office's two exports whose base is not attached and on none of the 5 whose
  is. A wall with no opening keeps the four-vertical-face gate, since the two
  such walls of the BIM4LCA housing with more have a `NetVolume` alone.
  Measured on the saved meshes rvt-rs reads (Measure 43115751418): 1,771 values
  read equal to Revit's over the 17 files whose Revit export writes base
  quantities, the held-out pairs among them, none different and none that Revit
  omits. The rule also writes 1,880 values where no Revit export can check
  them: 292 on Core Interior, RE1 Architecture and Einhoven, whose exports hold
  no base quantities, and 1,588 on 17 of Autodesk's sample saves of 2024 to
  2027.

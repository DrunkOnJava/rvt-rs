- **Nothing in Revit's own bodies or in the saved graphics tells which walls and
  mullions Revit leaves quantities off (RE-399, #1046).** On the BIM4LCA
  office's two files Revit writes a `NetVolume` alone on 28 of 607 walls as
  built (24 of 589 at permit), and the body Revit draws a wall with does not
  separate them from the walls with the full set: 304 of the 331 walls with a
  face-set body have it, and so do 275 of the 276 extrusions. The saved graphics
  of the 10,003 curtain mullions flag no primitive hidden or filtered, so they
  say nothing of the end faces Revit's `NetSurfaceArea` leaves out. For columns
  Revit's body and its quantity set agree on every column, which is RE-291
  read from Revit's side. The report names what stays unmeasured; nothing
  rvt-rs writes changes.

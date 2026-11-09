- **A body drawn from its saved mesh of flat faces gives its `NetVolume`, as
  Revit's export writes it (RE-387).** Revit's export takes a curtain mullion's,
  a curtain panel's and a column's `NetVolume` from the volume of its solid,
  and the mesh Revit saved for it is that solid, so where rvt-rs draws the
  body from the mesh (the default since #1191), every face of it is flat and it
  closes, the volume of the mesh is the quantity. On the BIM4LCA office it is
  Revit's on all 9,678 mullions and 260 panels drawn so as built (9,706 and 269
  at building permit), 1,883 and 260 of which had none. A body with a curved
  face, whose mesh is a tessellation, and an entity not measured get none; a
  column's `GrossVolume` stays out, since Revit omits it on 18 columns at
  permit that nothing tells from the 6 it writes it on; and `NetSurfaceArea`
  stays out: Revit leaves out the end faces of a mullion that its joins hide.

- **A wall's `NetVolume`, `NetSideArea` and, where its two sides agree,
  `GrossSideArea` and `GrossVolume` come from the mesh Revit saved for it, as
  Revit's export writes them (RE-388).** Revit's export takes them from the
  wall's solid, joined to the walls beside it and cut by what it hosts, and the
  saved mesh is that solid: its volume is `NetVolume`, the area of its largest
  vertical face less its holes `NetSideArea`, the area inside the outer loop of
  that face `GrossSideArea`, and the wall type's width times it `GrossVolume`.
  A wall gets them where every face of its mesh is flat, the faces close into
  one piece wound one way, and the quantity is not already written by a rule of
  its own (RE-270, RE-293); a gross area only where the two largest vertical
  faces, the wall's sides, face opposite ways and have the same outer-loop
  area, since Revit's choice between two sides that differ is not read. A wall
  with a curved face, a mesh that does not close, or two pieces gets none.

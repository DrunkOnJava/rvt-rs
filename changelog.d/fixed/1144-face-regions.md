- **A plane face split into regions that tile it is drawn whole (RE-329,
  #1144, #1247).** Revit saves a face that was split, for paint or material,
  as the face and a region face for each part, and rvt-rs refused any face
  with regions, which kept the whole element's record box. A plane face
  whose regions are plane faces on its own plane, each inside its outline,
  with one loop and none of their own, that together cover exactly its area,
  is now drawn as the one face it is; any other face with regions is refused
  as before. Doors and windows are drawn from their saved meshes by default,
  so this moves the default export of the Revit 2025 models 12_Rooms_Door_
  Windows to 17_Rooms_Stairs_Spiral, three windows each from their record
  box to their mesh, 18 in all. Against Revit's own export every one is
  exact (volume ratio 1.0000, Revit's vertices within 1.8e-6 ft of the mesh
  and the mesh's of Revit's), the census no longer lists regions as a cause
  in any file, and no other line of those six files' scores moves. No door
  or window of the BIM4LCA office, Core Interior or RE1 is kept as a box for
  regions, so none changes there. Not measured: a face with regions on any
  other surface, or with holes, which stay refused, and files other than
  these six; whether regions explain #1247's boxed doors and windows is for
  its reporter's census to say.

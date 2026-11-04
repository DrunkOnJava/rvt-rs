- **A plane face split into regions that tile it is drawn whole (RE-329,
  #1144, #1247).** Revit saves a face that was split, for paint or material,
  as the face and a region face for each part, and rvt-rs refused any face
  with regions, which kept the whole element's record box. A plane face
  whose regions are plane faces on its own plane, each with one loop and
  none of its own, listed once, is now drawn as the one face it is where the
  regions tile it: each lies inside its outline, shares no area with
  another, and together they cover exactly its area, measured on
  triangulations. Any other face with regions is refused as before. Doors
  and windows are drawn from their saved meshes by default, so this moves
  the default export of the Revit 2025 models 12_Rooms_Door_Windows to
  17_Rooms_Stairs_Spiral, three windows each from their record box to their
  mesh, 18 in all. Against Revit's own export every one is exact (Measure
  run 41790592582: volume ratio 1.0000, Revit's vertices within 1.8e-6 ft
  of the mesh and the mesh's of Revit's), the census no longer lists
  regions as a cause in any file, and no other line of those six files'
  scores moves. No door or window of the BIM4LCA office, Core Interior or
  RE1 is kept as a box for regions (census, run 41786625424), so none
  changes there. The whole Measure (run 41791824472) moves 15 of 111
  exports and no other: the six models above; 42 windows of Autodesk's
  Snowdon Towers architectural sample, from box to mesh (172 of 172 door
  and window bodies drawn, 130 before; no Revit export, so not scored); and
  eight held-out models, 88 windows in all, every one exact against Revit's
  export (counts only, run 41793911979). Not measured: a face with regions
  on any other surface, or with holes, which stay refused; whether regions
  explain #1247's boxed doors and windows is for its reporter's census to
  say.

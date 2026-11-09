- **The opening a door cuts in a second wall carries its door's `FireExit` in
  `Pset_OpeningElementCommon`, as Revit's export writes it (RE-386).**
  Revit's export gives an opening the properties of the element that cuts it,
  so where a project parameter names `FireExit` on its doors (the BIM4LCA
  office's does), each opening a door cuts in a further wall has the door's
  value beside the `Reference` it already had. The office's export writes it
  on every opening a door cuts (43 as built, 45 at building permit, all
  `false`); rvt-rs writes the openings that are in walls (25 and 33), equal to
  Revit's. An element with no such parameter, a cabinet or a curtain wall,
  gets none, as in Revit's export.

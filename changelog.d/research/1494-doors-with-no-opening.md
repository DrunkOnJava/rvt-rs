- **A door set in no wall is placed at its stored origin (#1494, RE-361).**
  Where Revit's export gives a door no opening, its `IfcLocalPlacement` origin
  is the origin of its stored transform (RE-357); rvt-rs now writes it there
  for a door it binds to no host wall, with the body and everything it
  carries where they were in the world. Measured on this change's Measure run
  once it has finished.

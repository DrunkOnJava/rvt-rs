- **A column drawn from its saved mesh no longer takes a taller record box's
  height as its `Length` (#1801).** On the Revit 2025 to 2027 saves of
  Autodesk's `rme_advanced` and `rme_basic` samples, 131 columns in each file
  have a record box 3.5, 7.3, 10.6 or 7.0 m tall around a mesh 3.2 to 3.5 m
  tall, the same mesh their 2024 saves hold, and the round ones, whose mesh
  quantities are not read, wrote the box's height. A column whose `Length` is
  its box's height now takes its mesh's height where the mesh is shorter. No
  Revit export covers these files: they are held to their 2024 saves.

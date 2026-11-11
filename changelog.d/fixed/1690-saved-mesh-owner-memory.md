- **The saved-mesh pass no longer holds every wall's graph at once (#1690).** It
  read every owner's graph first and held all of them until the last batch was
  meshed, and since RE-388 the owners include every wall, whose graph is about
  1.6 MiB (#1429): on a Revit 2023 to 2027 file the default export took 1.4 to
  4.4 times the peak memory of 0.17.0 (Core Interior 305 to 647 MiB, Snowdon
  Towers Architectural 1,072 to 3,964). The owners that name nothing, walls
  among them, are now let go after that first read and read again in one
  further pass that meshes a buffer of about 20,000 graph objects at a time. No
  body changes. Measured numbers follow the run.

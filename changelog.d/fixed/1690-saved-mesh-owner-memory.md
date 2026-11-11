- **The saved-mesh pass holds one batch of owners' graphs at a time (#1690).** It
  read every owner's graph first and held all of them until the last batch was
  meshed, and since RE-388 the owners include every wall, whose graph is about
  1.6 MiB (#1429): on a Revit 2023 to 2027 file the default export took 1.4 to
  4.4 times the peak memory of 0.17.0 (Core Interior 305 to 647 MiB, Snowdon
  Towers Architectural 1,072 to 3,964). Each owner is now read once for what
  its graph names and how large it is, and again with the batch it is meshed
  in; owners that name no symbol, walls among them, are batched by their graph
  objects as symbols are. No body changes. Measured numbers follow the run.

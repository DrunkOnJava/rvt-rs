- **A Revit 2024 pipe is drawn as its cylinder (#869).** Pipe cylinders and
  the ports at their ends were read on Revit 2025 only. On the BIM4LCA
  office, a Revit 2024 project, all 12 pipes on each of its two files are
  now Revit's cylinder, with its radius, axis ends and length, where they
  were boxes, and their 24 ports sit at their ends with Revit's axis. A
  2024 pipe's inner diameter is not read, so it writes no `InvertElevation`
  rather than one from its axis alone. `pipe_bodies_vs_ifc` compares each
  file in its own site frame and reads a profile Revit writes as four arcs.

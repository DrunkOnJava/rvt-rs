- **The owners that name nothing (walls, beams, columns) are meshed 5,000 graph
  objects at a time, not 20,000, which takes 16% off Core Interior's peak memory
  and 3% off the BIM4LCA office's (#1694).** The saved-graphics pass reads those
  owners in one pass and meshes a buffer of them at a time; the buffer, about
  95 MiB at 20,000 objects, was the pass's peak on a file whose walls fill it. On
  one runner, `main` against the change: Core Interior 516 to 434 MiB, the office
  as built 1,249 to 1,205 and at permit 1,375 to 1,329, every other file 0.96 to
  1.05 of main's (the runs' noise) and export time 0.91 to 1.03. A file whose
  owners never fill the buffer, such as `rst_basic`, is not moved. The exports
  are byte for byte as before.

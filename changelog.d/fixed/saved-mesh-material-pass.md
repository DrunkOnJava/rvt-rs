- **Exports with saved meshes skip a pass over the file nothing used,
  and run up to half again as fast.** After reading doors' and windows'
  saved meshes (on by default), the saved-graphics reader made a second
  pass over every element's graphics to resolve render materials into two
  fields that no exporter, binding or probe read. That pass, and the
  unused saved-graphics glTF writer that was their only reader, are gone.
  Exports are byte-identical on all 32 reference models, and `rvt-ifc`
  takes 3.5 s instead of 5.8 s on Core Interior and 17.9 s instead of
  24.7 s on the BIM4LCA office (medians of three rounds on one runner);
  files whose doors and windows have no saved meshes are unchanged.

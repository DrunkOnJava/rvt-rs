- **Exports with saved meshes skip a pass over the file that nothing
  used.** After reading doors' and windows' saved meshes (on by default),
  the saved-graphics reader made a second pass over every element's
  graphics to resolve render materials into two fields that no exporter,
  binding or probe read. That pass, and the unused saved-graphics glTF
  writer that was their only reader, are gone. Exports are byte-identical
  on all 32 reference models, and `rvt-ifc` takes 24% to 52% less time on
  those with saved door and window meshes: 3.5 s instead of 5.8 s on Core
  Interior, 17.9 s instead of 24.7 s on the BIM4LCA office as built
  (medians of three rounds on one runner).

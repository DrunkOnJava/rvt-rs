- **Exports with saved meshes skip a pass over the file nothing used.**
  After reading doors' and windows' saved meshes (on by default), the
  saved-graphics reader made a second pass over every element's graphics
  to resolve render materials into two fields that no exporter, binding
  or probe read. That pass, and the unused saved-graphics glTF writer
  that was their only reader, are gone; exports are unchanged.

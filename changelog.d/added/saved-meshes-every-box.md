- **Saved meshes replace a family instance's turned box, and decode
  ellipses, curved ruled surfaces and rings that repeat their first point
  (RE-385).** A family instance drawn as its record box turned to its angle
  (RE-87) is now replaced by its saved mesh, as a record box is. The
  saved-graphics reader reads a `GEllipse` as a surface profile, a
  `RuledSurf` between arcs and ellipses as well as lines, and a trim ring
  that repeats its first point (a conduit's end cap). On Snowdon Towers
  Electrical (Revit 2024 and 2025) 18 of the 2,428 elements stay boxes,
  against 300; 16 of them are art-glass pendant drops whose nested instances
  are forbidden the targets 10, which stay refused. Architectural draws
  2,508 bodies from saved meshes, against 2,377. Every mesh these changes
  add fits inside the box it replaces, on the 13 models probed. The default
  export takes up to 17% longer where it draws more meshes (Snowdon Towers
  Electrical 2024, 11.7 s to 13.7 s).

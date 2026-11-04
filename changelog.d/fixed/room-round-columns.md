- **A room's outline takes off the round columns its stored solid holds
  (#1354, RE-334).** The walk that reads a room's outline from its stored
  solid (RE-101) read only planar wall faces, so a vertical cylinder, a
  round column, was ignored: the room kept the column's area, and a room
  with columns against its walls kept its record box. The walk and the
  resolved span (RE-316) now read the arcs of vertical cylinders that reach
  the floor, as chords within 0.0005 ft, and a room with more than 4,096
  plan segments is left to its box.
  - On `rac_advanced`'s saves of 2015 to 2020, against the public IFC2X3
    export of the project, all 91 rooms are within 0.1% in area and 0.01 ft
    in outline, and 48 spaces have as many voids as Revit's (Measure run
    41900428551): before, 42 of the 85 rooms drawn from their stored solid
    were, and 6 kept their box. That export is the only Revit export of a
    room with a round column; the exports of the 33 other models measured
    (the BIM4LCA office, Core Interior, RE1, the R2025 models) are byte for
    byte as before (run 41899794330), so none of them has one.
  - Measure's `racadv` input scores the 2015 to 2020 saves against that
    export, fetched for the run and never stored or cached.
  - Not measured: a Revit IFC4 export of a room with a round column, and
    Revit's own room area for these rooms.

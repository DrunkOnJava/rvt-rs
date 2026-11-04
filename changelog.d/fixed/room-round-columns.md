- **A room's outline takes off the round columns its stored solid holds
  (#1354, RE-334).** The walk that reads a room's outline from its stored
  solid (RE-101) read only planar wall faces, so a vertical cylinder, a
  round column, was ignored: the room kept the column's area, and a room
  with columns against its walls kept its record box. The walk now reads the
  arcs of vertical cylinders that reach the floor first, as chords within
  0.0005 ft, and where that does not close it reads the faces as before, the
  columns ignored; the resolved span (RE-316) reads them too, but a span with
  a cylinder closes undirected or not at all, since an arc's direction is not
  read. A room with more than 2,048 plan segments is left as before.
  - On `rac_advanced`'s saves of 2015 to 2020, against the public IFC2X3
    export of the project, all 91 rooms are within 0.1% in area and 0.01 ft
    in outline (Measure run 41909372124): before, 42 of the 85 rooms drawn
    from their stored solid were, and 6 kept their box. 48 spaces have as
    many voids as Revit's, on all six saves (read outside Measure, run
    41900428551). A public Revit 2018 IFC4 export of `rac_basic` draws the
    one column of its room 101 as a void too, and the reader moves that room
    from 1.710% to 0.628% over its area (read outside Measure, run
    41909414507). The exports of the 33 other models measured (the BIM4LCA
    office, Core Interior, RE1, the R2025 models) are byte for byte as before
    (run 41909377642), and so are the 20 held-out and BIMcollab models
    (run 41909388707), so none of them has such a room.
  - Measure's `racadv` input scores the 2015 to 2020 saves against that
    export, fetched for the run and never stored or cached; only its summary
    counts are uploaded.
  - Revit's own area for `rac_basic` room 101 (its `GSA Space Areas`,
    781.08 ft²) is the geometry with the column taken off, as the schedule
    area now is; 4.95 ft² of the reader's 0.628% are in the room's outer
    ring, which this does not change and is not explained.
  - On Autodesk's samples (run 41909382808) the six rooms with columns
    against their walls take an outline on all 13 `rac_advanced` releases,
    2015 to 2027, and 2 of Snowdon Towers architectural's 7 boxed rooms do;
    nothing scores the 2021 to 2027 saves or Snowdon's two rooms, whose
    Revit IFC4 export holds no `IfcSpace`.
  - Not measured: Revit's own area for the `rac_advanced` rooms (its export
    carries none), a concave cylinder, a short column, and a prism room's
    `NetVolume` with columns (written for Revit 2024 and 2025 only).

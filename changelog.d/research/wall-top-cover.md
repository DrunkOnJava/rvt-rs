- **Revit cuts a wall's top only where the floor covers it (RE-267, #1067).**
  The 5 walls per stage that RE-258 found Revit leaves whole are cut over the
  part of their plan the floor covers and left at full height over the rest,
  so the highest point does not move: on the BIM4LCA office the area of
  Revit's body at the wall's full height equals the wall's plan outside the
  floor on 79 of 83 walls as built and 83 of 87 at permit. A rule that cuts a
  top where the floor named by its top join or a cut pair covers the wall's
  whole plan gives 64 of the 65 cut tops as built and 70 of 70 at permit,
  and cuts no wall Revit leaves whole; one wall's 0.085 ft stub past the
  floor's edge is cut by Revit and unexplained, and a part-covered wall
  needs a stepped top, which this export does not draw.
  `tools/re/wall_top_cover_vs_ifc.py` and
  `probes/examples/probe_1067_native_dump.rs` are the measurement.

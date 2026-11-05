- **Revit 2026 and 2027 projects draw beams along their axes, stair runs
  and tread landings as their steps and treads, and floors, ceilings and
  slabs from their sketch lines, with shaft voids cut from floors and
  ceilings (#1190, RE-341).** Beams are drawn as on 2024 (on 2025 they are
  not), runs from their native records as on 2025 with the stair property
  sets and quantities read from them, tread landings as on 2024 (a
  monolithic landing stays its box; on 2025 neither is drawn), and the
  sketch lines' exact ends, curves and Sketch curve lists are read as on
  2024 and 2025. A roof there keeps the record-box solve as on `main`,
  without its slope, so a sloped roof can still be drawn as a level block.
  **No beam, landing, stair run, sketch outline or shaft void of a 2026 or
  2027 file is scored against a Revit export**; each is held to the same
  Autodesk project's earlier save instead (`tools/re/bodies_across_saves.py`,
  which Measure runs and fails on), and that cannot catch an error the
  earlier save shares; it compares bodies, not quantities or property sets.
  No 2026 or 2027 held-out model holds a beam, so the beam reader has run
  only on two Autodesk samples saved from earlier releases. Against Revit's
  own export only one stair flight is scored, on a held-out 2026 model and a
  held-out 2027 one (property values equal 209 to 224 of 352 and of 354,
  quantities 21 to 23 of 322 and 17 to 19 of 367; Measure run 41988011289,
  head 1d5bbf03). In run 42023546789 (head d3635bbb) 103 of 111 exports are
  unchanged and 8 change, all 2026 or 2027; the check reads PASS on all six
  samples, and the exports' `rules:` findings are 0. The one outline the 2024
  and 2025 saves disagree on, a floor, is the 2025 save drawing its arcs as
  chords (24 vertices against 170): the arc readers are not enabled on 2025
  (#1440).

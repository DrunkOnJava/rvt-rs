- **Revit 2026 and 2027 projects draw beams along their axes, stair runs
  and landings as their steps and treads, and floors, ceilings, slabs,
  footprint roofs and shaft voids from their sketch lines (#1190, RE-341).**
  Beams are drawn as on 2024 (on 2025 they are not), runs from their native
  records as on 2025 with the stair property sets and quantities read from
  them, landings as on 2024 (on 2025 they are not), and the sketch lines'
  exact ends, curves and Sketch curve lists are read as on 2024 and 2025.
  Measured against the same Autodesk samples saved by an earlier release (the
  contributor's local pairs):
  `rst_basic` 2026's and 2027's 345 beams, 9 beam boxes and stair run are
  their 2024 save's, and `rac_advanced`'s 37 beams and 5 landings are its
  2024 save's, and its 10 stair runs and `rac_basic`'s 2 are their 2025
  saves'. Against Revit's own export only one stair flight is scored, on a
  held-out 2026 model and a held-out 2027 one (property values equal 209 to
  224 of 352 and of 354, quantities 21 to 23 of 322 and 17 to 19 of 367;
  Measure run 41988011289, head 1d5bbf03); no beam, landing, sketch outline
  or roof is. Of 111 exports in that run, 9 change, all 2026 or 2027, and
  102 are unchanged. Sloped
  roofs, spiral runs, stair names, railing heights and lengths, a wall's
  profile openings, IFC export overrides and the opening index stay unread
  on 2026 and 2027. The head that reads Sketch curve lists on 2026 and 2027
  and leaves roofs out is not yet measured.

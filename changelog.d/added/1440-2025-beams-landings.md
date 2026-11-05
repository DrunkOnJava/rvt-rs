- **Revit 2025 projects draw beams along their axes and tread landings as
  their treads, and read a sketch line's arcs (#1440, RE-345).** Beams are
  drawn as on 2024, 2026 and 2027, tread landings as on 2024 (a monolithic
  landing stays its box), and a floor, ceiling or roof whose sketch holds
  an arc takes the arc where the 2025 save drew a chord: rac_advanced's
  floor has the 170 vertices of its 2024 save, not 24, and its roof and
  ceiling equal the 2024 save's. **No Revit export checks any of it on
  2025**: the 25 models with Revit's own 2025 exports (puzzbobb's 24 and
  RE1 Architecture) hold no beam, no tread landing, no shaft opening and no
  floor, ceiling or roof with an arc, and their exports are byte for byte
  unchanged. Each is held to the same Autodesk project's 2024 save instead
  (`tools/re/bodies_across_saves.py`, which Measure runs and fails on): 391
  beams and 5 tread landings of rac_advanced and rst_basic are equal per
  element, and of the 7 elements whose sketch holds an arc, 2 (a floor and
  a roof) are compared as drawn outlines and equal, the other 5 being
  record boxes or having no body. That cannot catch an error the 2024 save
  shares, and it compares bodies, not quantities or property sets; none of
  the 19 held-out and spent models changes (Measure run 42091457764, head 830a4d94).
- **Shaft openings are cut from floors, ceilings and roofs on Revit 2024
  only, and no longer on 2026 and 2027 (#1440, RE-345).** #1426 had turned
  the cut on for 2026 and 2027. The only shafts measured on a 2024 or 2025
  Autodesk sample, two in rac_advanced, cut nothing, so the check against
  the same project's earlier save cannot catch a bad cut, and no file of
  2025 to 2027 is known to exercise one. On those releases the floors,
  ceilings and roofs stay whole, and the export's diagnostics count the
  shaft openings it did not cut as the `skipped` item `shaft_opening_not_cut`.
  No export changes by it: in Measure run 42091457764 (head 830a4d94) of
  111 exports, 108 are byte for byte `main`'s, where the cut is still on
  for 2026 and 2027 (the 3 that move are the Autodesk 2025 samples), among
  them all 10 Autodesk samples of 2026 and 2027 and 5 held-out models of
  those years; the only 2026 and 2027 shafts counted, 2 in each
  rac_advanced, cut nothing.

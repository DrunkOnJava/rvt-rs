- **Revit 2025 projects draw beams along their axes and tread landings as
  their treads, and read a sketch line's arcs, circles and shaft lines
  (#1440, RE-345).** Beams are drawn as on 2024, 2026 and 2027, tread
  landings as on 2024 (a monolithic landing stays its box), and a floor,
  ceiling or roof whose sketch holds an arc takes the arc where the 2025
  save drew a chord: rac_advanced's floor has the 170 vertices of its 2024
  save, not 24, and its roof and ceiling equal the 2024 save's. **No Revit
  export checks any of it on 2025**: the 24 models with Revit's own 2025
  exports (puzzbobb's 23 and RE1 Architecture) hold no beam, no tread
  landing, no shaft opening and no floor, ceiling or roof with an arc or a
  circle, and their exports are byte for byte unchanged. Each is held to the
  same Autodesk project's 2024 save instead (`tools/re/bodies_across_saves.py`,
  which Measure runs and fails on): 391 beams and 5 tread landings of
  rac_advanced and rst_basic are equal per element, and of the 7 elements
  whose sketch holds an arc, 2 (a floor and a roof) are compared as drawn
  outlines and equal, the other 5 being record boxes or having no body. That
  cannot catch an error the 2024 save shares, and it compares bodies, not
  quantities or property sets; none of the 19 held-out and spent models
  changes (Measure run 42062038214, head 6ea88323).

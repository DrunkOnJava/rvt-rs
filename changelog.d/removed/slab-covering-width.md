- **A floor's and a covering's `Width` (#1046, RE-294).** Revit 2024's
  exporter (24.0.5) writes `Qto_SlabBaseQuantities.Width` on a compound
  floor and `Qto_CoveringBaseQuantities.Width` on a ceiling and on a wall
  it exports as a covering; the exporters that wrote two held-out models (a
  Revit 2025 and a Revit 2026 file) write none on them, so rvt-rs wrote
  values Revit's export omits there. Where Revit's exporters disagree on a
  value, rvt-rs leaves it out, and these are no longer written, including
  the ceilings' `Width` of 0.11.0 (RE-250): the BIM4LCA office loses 113
  quantities that were equal as built and 89 at building permit (they
  become missing, none different), and the held-out models go from 2 and 3
  values Revit's export omits to none (Measure run 41204218380).

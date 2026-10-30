- **Where Revit's `LoadBearing`, a slab's `IsExternal` and
  `ThermalTransmittance` come from, and why none is written on Revit 2015 to
  2022 (RE-300, #1158).** A wall's `LoadBearing` is its record's Structural
  box (`m_isStructuralSignificant`: 59 of 59 walls of the flowbim house, both
  values), a slab's is the saved parameter `-1001954` (8 of 8, both values)
  and its `IsExternal` the type's Function (`-1001006`); each is exact on the
  held-out models of Revit 2023 to 2026. BIMcollab's Revit 2015 export, the
  only one of a 2015 to 2022 file, writes `LoadBearing` from a parameter
  named for it on every wall and true on slabs whose parameter is 0, so it
  confirms none, and nothing is written there. `ThermalTransmittance` is no
  field of any record, and beams' `Span` and `Slope` need an axis only the
  2024 layout reads. `probe_re285_flag_candidates`, `probe_re300_native_fields`
  and their scorers are the measurement.

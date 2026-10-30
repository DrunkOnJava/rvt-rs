- **`Pset_BuildingCommon.NumberOfStoreys` only where it cannot be wrong
  (RE-263, #1097).** Revit's exporter counts the storeys that contain an
  element up to IFC 26.0 and every storey from IFC 26.1, and a `.rvt` does not
  say which will export it. rvt-rs now writes the count only where every
  storey contains an element, so both give the same number; elsewhere the
  property is left out (the maintainer, 2026-10-29).

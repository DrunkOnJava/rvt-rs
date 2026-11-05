- **A 2027 wall's record holds a second line, its previous location line,
  which is why 8 host walls of `rac_advanced` returned no native line
  (RE-360, #1475).** On Revit 2027's save of `rac_advanced` 18 of the 180
  `SWall` records hold two `GLine` where the 2024 to 2026 saves hold one; the
  second is owned by `WallRunTimeData.m_prevLocLine`, and the one
  `VWallDriver.m_pCrv` names is the location curve, with the 2024 save's
  length on all 180 walls. The 8 host walls of the 14 doors and windows left
  on the default axes (RE-347) are among the 18. The same member is in other
  records of the 2024 to 2026 saves (`rac_basic` 5 walls, Snowdon Towers
  Architectural 2), so every rule that reads these walls also reads walls
  there; no reader changes, the numbers are in the report. Measured on
  Autodesk's sample projects of 2024 to 2027 (no Revit export), Measure run
  42129732742: the probe `probe_re360_wall_location_curve` prints each wall's
  curves, their owners and what `m_pCrv` names.

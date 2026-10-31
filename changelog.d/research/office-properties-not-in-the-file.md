- **Three BIM4LCA office properties the file does not decide (W7, RE-277).**
  Columns' `Pset_ColumnCommon.Slope` (73 of 279 as built, 101 of 345 at
  permit, all 0.0) comes from the last geometry object Revit's exporter sees
  for the column, and the file does not fix it: six columns have a native
  record identical in both files and Revit gives them the property only at
  permit, and twelve identical columns on one level split 5 to 7 by whether a
  join is user-set. The building's `Reference` (`Project Information`) is a
  string the file does not hold. The door openings' 140 `Reference` and 31
  `FireExit` values are on the openings Revit writes where a door cuts a
  further wall, which rvt-rs does not write. None is written. The probe
  `probe_w7_office_gaps` lists each Level's, beam's, column's and pipe's
  native record for the office and the R2025 models.

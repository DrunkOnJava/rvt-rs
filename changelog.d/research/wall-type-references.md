- **Which type a wall's records name (RE-253, #1048).** On the held-out
  model where one wall's `Width` differs from Revit's, every reading of the
  saved file (the wall's reference list, its native record, its type's data
  copies, the distance between its record's faces) gives it the 200 mm type
  rvt-rs picks, while Revit's export, made hours after the last save, gives
  it a 300 mm type the file also holds: most likely an unsaved change.
  Count-only probes and a scorer of the export's time against the save's
  run on held-out models.

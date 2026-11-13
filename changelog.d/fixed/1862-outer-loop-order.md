- **Two exports of one file write the same wall `GrossSideArea` and
  `GrossVolume` to the last bit (#1862).**
  Each boundary loop of a wall's side was summed from a vertex taken in hash
  order, so the area moved by up to 2e-14 of its value from one export to the
  next. `rvt-diff --elements` comparing a file's export with itself listed 4
  to 117 walls as changed on seven of eleven models.

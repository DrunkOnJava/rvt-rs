- **A Revit 2015 to 2023 project whose element frames carry no ElementId of
  their own now exports those elements.** Revit 2018's `ALD26 Before` (Revit's
  own export holds 1,394 elements) exported none (#1261). Most of its frames in
  `Partitions/0` do not open with a declared `u32` ElementId; they sit inside
  their element's partition record, as 2024 and later frames do (RE-35). Those
  records are `u32` ElementId, `u32` size, the tag of `ElementHeader`, a count,
  the body and a 12-byte trailer that echoes the size, in runs with stretches
  between them that hold none; the 2024 walk's flag test stops at the first
  record with another flag word. A placed-instance frame with no declared id
  now takes the id of the declared record around it when its own reference list
  names that id. A curtain panel frame that opens with a count rather than
  `ff ff ff ff` is left out, as 96 of the 99 on that file are the shells of
  panels Revit's export does not write (RE-315). Elements Revit's export
  holds whose ElementIds have no record in any partition stay missing.

- **A Revit 2015 to 2023 project whose element frames carry no ElementId of
  their own now exports those elements, on their storeys.** Revit 2018's
  `ALD26 Before` exported none of the 1,394 elements and spaces in Revit's own
  export (#1261): most of its frames in `Partitions/0` do not open with a
  declared `u32` ElementId, but sit inside their element's partition record, as
  2024 and later frames do (RE-35). Those records are `u32` ElementId, `u32`
  size, the tag of `ElementHeader`, a count, the body and a 12-byte trailer that
  echoes the size; the 2024 walk's flag test stops at the first record with
  another flag word, after 115 of this file's 12,626. A placed-instance frame
  with no declared id now takes the id of the declared record around it when
  its own reference list names that id, and so does a Level frame, which gives
  the file its 7 storeys. The file now writes 1,136 of Revit's 1,321 elements
  by ElementId, each in Revit's class or IFC4's for its IFC2X3 one, none that
  Revit's export lacks, 1,073 on Revit's storey. A curtain panel frame that
  opens with a count rather than `ff ff ff ff` is left out, since 96 of the 102
  on that file are the shells of panels Revit does not write. The 185 elements
  still missing are 178 conduits, conduit fittings and gutters, whose
  categories are not yet recovered, 4 panel walls and 3 assemblies (RE-315).
  Files whose frames all open with their id export as before.

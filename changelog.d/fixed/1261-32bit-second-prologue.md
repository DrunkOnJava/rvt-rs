- **A Revit 2008 to 2023 project whose element frames carry no ElementId of
  their own now exports those elements, on their storeys (measured on one
  Revit 2018 project).** Revit 2018's
  `ALD26 Before` exported none of the 1,394 elements and spaces in Revit's own
  export (#1261): most of its frames in `Partitions/0` do not open with a
  declared `u32` ElementId, but sit inside their element's partition record, as
  2024 and later frames do (RE-35). Those records are `u32` ElementId, `u32`
  size, the tag of `ElementHeader`, a count, the body and a 12-byte trailer that
  echoes the size; the 2024 walk's flag test stops at the first record with
  another flag word, after 115 of this file's 12,626. A placed-instance frame
  with no declared id now takes the id of the declared record around it when
  its own reference list names that id, and so does a Level frame that is a
  `Level` with a name block in its stream, which gives the file its 7 storeys.
  The file now writes 1,136 of Revit's 1,321 elements by ElementId, 1,127 of
  them in Revit's class or IFC4's for its IFC2X3 one (9 curtain walls come out
  as walls), none that Revit's export lacks, 1,073 on Revit's storey. A curtain
  panel frame that opens with a count rather than `ff ff ff ff` is left out,
  since 96 of the 102 on that file are the shells of panels Revit does not
  write. The 185 elements still missing are 178 conduits, conduit fittings and
  gutters, whose categories are not yet recovered, 4 panel walls and 3
  assemblies (RE-315). Files whose frames all open with their id export as
  before.
- **Revit 2016 to 2023 exports leave out the elements Revit's own export
  leaves out by phase and by design option.** Only 2024 and later applied the
  export phase (RE-184) and the primary design option (RE-40). A Revit 2018
  project wrote 198 pipes, fittings and other elements that Revit's export does
  not hold, and writes none now; the counts cannot say whether the phase or the
  design options removed them, and the held-out file is not opened to find
  out. The phase is read as RE-281 reads it. Design options are read as on 2024
  with `u32` ids: on the `rst_basic_sample_project` of each of 2016 to 2023 the
  one `Mass` in a non-primary option is found and counted, as the 2024 and 2025
  copies of the model count it, and leaves the production counts of the
  diagnostics without changing the exported IFC. Revit 2008 to 2015 keep every
  element (RE-315).

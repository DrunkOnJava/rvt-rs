- **Values a project's own property set file places are out of 1.0's
  scope (RE-239, #35).** On the BIM4LCA office, Revit's export writes
  `Pset_DoorWindowGlazingType` on walls, curtain walls and coverings and
  takes rooms' `OccupancyType` from their `RoomTypeCode` parameter, neither of
  which revit-ifc's own definitions do: the export's user-defined property set
  file chose them, and the model does not store that file. A probe and
  `tools/re/mapped_parameters_vs_psets.py` trace each value to the element's
  saved parameter; W7 of the 1.0 criteria now leaves these values out and
  RE-239 lists them per reference model (the maintainer's decision,
  2026-10-26).

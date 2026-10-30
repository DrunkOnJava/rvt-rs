- **A stair's `TreadLengthAtInnerSide` and flight set (W7, RE-287, #1186).**
  Revit's export writes on each `IfcStair` a `Pset_StairCommon.
  TreadLengthAtInnerSide` and, by revit-ifc's matching of a set to an entity
  by the start of its name, a `Pset_StairFlightCommon` holding the sums of its
  flights' risers and treads and their common riser height, tread length,
  nosing length and tread lengths at offset and at the inner side. rvt-rs
  writes the first always (the stair's tread depth, as a flight's) and the
  second with `rvt-ifc --revit-set-placement`: 111 values on each of the
  BIM4LCA office's two files (14 stairs; the spiral stair's tread count is
  not read) and 8 on each R2025 stair model, 7 on the spiral one, all equal
  to Revit's export's, none different. No other model's export changes. The
  mechanism in the exporter that fills the container's flight set is not read:
  the rule is measured on 14 and 5 stairs of one to three flights.

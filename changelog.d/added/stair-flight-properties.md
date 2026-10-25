- **Stair flights' treads, tread lengths, nosing and run type.** On Revit
  2024 projects each flight's `Pset_StairFlightCommon` carries its number of
  treads (its risers, one fewer when it begins and ends with a riser),
  `TreadLengthAtOffset` and `TreadLengthAtInnerSide` (the stair's tread
  depth), its run type's nosing length and the run type's name as its
  `Reference`, and each stair's `Pset_StairCommon` its `TreadLengthAtOffset`
  and the nosing length its runs share. On the BIM4LCA office these are
  Revit's on 34 or 35 of the 35 flights rvt-rs writes and 14 of 14 stairs,
  none different. `WalkingLineOffset` is not written: Revit's export gives it
  as half the run's width in feet read as millimetres.

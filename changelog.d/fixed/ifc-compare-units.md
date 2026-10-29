- **`rvt-ifc-compare` reads each file in its own length unit.** It
  compared storey elevations and the bounding box as the numbers each file
  writes, so rvt-rs's metres against Revit's feet or millimetres differed
  by 3.28 or 1,000 times on every storey. Both sides are now converted to
  metres by the file's own `LENGTHUNIT` (`compare::length_unit_scale_to_metres`),
  and RE1's storeys come out the same as Revit's.

- **More roof-joined walls' side areas and volumes (#1046, RE-290).** The
  longest vertical face of Revit's solid for a plain wall is as long as the
  centreline cut back where the wall meets the walls joined at its ends,
  which rvt-rs reads wherever both ends agree with the record's join reaches.
  A wall joined to roofs, floors or footings that lie clear of it, whose
  body is its record's box, now carries `GrossSideArea`, `NetSideArea`,
  `GrossVolume` and `NetVolume` when that length is its box's long side, not
  only when the record confirms its `Length`. They are Revit's on all 29 such
  walls of the BIM4LCA office as built and 36 at building permit, none
  different and none that Revit's export leaves out: the office goes from
  28,162 to 28,278 of the 39,388 quantities equal as built (Measure run
  41182651693).

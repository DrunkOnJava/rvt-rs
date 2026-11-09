- **A wall whose edited elevation profile closes below its record box ends at the
  profile's highest point, and its `Height` is the profile's (#1600, RE-378).**
  The BIM4LCA office's terrain files hold a wall (1387731) whose profile closes
  3.6765 ft below its box over its whole 173.9 ft: Revit's body ends at the
  profile's highest point, 196.5223 ft, where rvt-rs drew the box's 200.1988 ft,
  because its openings are cut from the profile's own bounding rectangle and left
  the strip up to the box solid, and it wrote the box's 5.520605 m as the wall's
  `Qto_WallBaseQuantities.Height` where Revit writes the profile's 4.4 m. The wall's
  body is now drawn to the profile's highest point, and it carries `ProfileTopSource`
  (`wall_top_at_profile_top`), counted in `exported.wall_profile_top_sources`. The
  rule rests on that one wall, so it is as narrow as the wall allows: the record
  names nothing joined to or cutting the wall, the profile spans the whole of it
  and starts at its base, and its highest point lies below the box. A wall that
  goes on past its profile keeps the box's top, as `rac_basic`'s wall 493612 does,
  whose box holds the profile's line carried on. Revit 2024 only.

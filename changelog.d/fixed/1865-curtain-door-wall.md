- **A door or window with no curtain grid found whose reference list names
  exactly one curtain wall is a part of that wall when the element data of an id
  in its list names that wall too, as a mullion's or a panel's grid does
  (#1865, RE-452).** Doors and windows took a curtain wall only through the
  curtain grid their record names, whose offsets (RE-72) are not found on
  Revit 2027, so the curtain door 216844 of Autodesk's `rac_advanced` was
  aggregated into the curtain wall 162023 on the 2025 and 2026 saves and stood
  alone, contained in the storey, on the 2027 save, and lost the `IsExternal`
  its wall gives. The data fallback RE-449 added for mullions never reached
  openings. Naming the wall alone is not enough for an opening: a door or window
  in an ordinary wall can name a curtain wall in its list too.

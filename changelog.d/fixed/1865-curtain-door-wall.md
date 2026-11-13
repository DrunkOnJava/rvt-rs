- **A door or window with no curtain grid found whose reference list names
  exactly one curtain wall is a part of that wall when the element data of an id
  in its list names that wall too, as a mullion's or a panel's grid does
  (#1865, RE-452).** Doors and windows took a curtain wall only through the
  curtain grid their record names, whose offsets (RE-72) are not found on
  Revit 2027, so the curtain door 216844 of Autodesk's `rac_advanced` was
  aggregated into the curtain wall 162023 on the 2025 and 2026 saves and stood
  alone, contained in the storey, on the 2027 save, and lost the `IsExternal`
  its wall gives; it is a part of 162023 on 2027 now. The data fallback RE-449
  added for mullions never reached openings. Naming the wall alone is not enough
  for an opening: it put one door of a Revit 2018 model in a curtain wall where
  Revit's export leaves it in an ordinary one. Against Revit's export of the four
  BIM4LCA offices, 184 of the 268 doors it aggregates into a curtain wall and
  rvt-rs did not are now in the same wall, none in another, and 21 in each
  office are still left out; 21 of 146 exports change (the offices, the
  `rac_advanced`, `rac_basic`, `rme_advanced` and `rme_basic` samples of 2024 to
  2027, and Snowdon Towers Architectural), none of Revit 2015 to 2023.

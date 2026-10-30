- **Walls' `IsExternal` and the parameters named for a standard property
  on Revit 2015 to 2023, experimentally (RE-283, #1158).** A wall's type
  Function (`BasicWallType.m_function`) is read from its native records on
  those releases, and the project and shared parameters named `IsExternal`,
  `<set>.IsExternal` or for a property of one of IFC4's sets are read from the
  saved parameters, so a wall's `IsExternal` and those properties are written
  as on 2024 and 2025. The Function is equal to the same project's later
  copy on 1,982 of 1,983 walls over 24 pairs of Autodesk's samples (the other
  is an edit between the saves). Revit's own exports agree on the 114 walls of
  BIMcollab's Revit 2015 example (`IsExternal` and `LoadBearing`, which two
  parameters on its walls give) and the 4 walls of modelo_bim, a Revit 2023
  file (`IsExternal`, from the Function), none different. Doors, windows and
  slabs get none: their Function is not read on these releases.

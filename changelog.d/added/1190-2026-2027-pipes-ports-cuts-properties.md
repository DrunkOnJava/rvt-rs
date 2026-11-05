- **Revit 2026 and 2027 projects draw pipes as their cylinders, place family
  instances' ports, cut walls to their doors' and windows' outlines, and
  write `IsExternal`, railing heights and lengths, stair names, rooms'
  height and net volume and the pipe and duct quantities, as 2025 projects
  do (#1190, RE-343).** Ten readers read the 2025 records these releases
  write; only their release lists change. **None of it is scored against a
  Revit export of a 2026 or 2027 file**; it is held to the same Autodesk
  sample saved by Revit 2025, and that cannot catch an error the 2025 save
  shares. On `rme_advanced` and `rme_basic` 2026 and 2027 the bodies that
  differ from the 2025 save fall from 490 and 493 to 0, on `rac_advanced`
  from 56 to 10, and every port of those three and `rac_basic`, 14,095 in
  all, is the 2025 save's.
  Every 2015 to 2025 file exports as before.

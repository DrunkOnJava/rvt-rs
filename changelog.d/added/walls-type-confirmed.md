- **Walls whose saved record states their type, in the export diagnostics
  (#1048, RE-253).** `rvt-ifc --diagnostics` lists the walls whose native
  record names the type the export gives them (`decoded.walls_type_confirmed`).
  `tools/re/qtos_vs_ifc.py` counts a wall's differing `Width` apart, where
  Revit's export gives it another type, only on such a wall, so a wall whose
  type rvt-rs picks wrong stays scored.

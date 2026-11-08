- **A family instance whose type draws nothing visible is left out, as Revit's export
  leaves it out (#1496, RE-363).** It is counted in the diagnostics as
  `element_record_family_type_draws_nothing`. Doors and windows are never left out this way.

- **A curtain panel that is the host panel of another curtain wall's grid is
  left out on Revit 2024, as Revit's export leaves it out (#1975).** Snowdon
  Towers Architectural's 23 host panels were written as `IfcPlate` that Revit
  writes nothing for; the export diagnostics count them as
  `element_record_curtain_host_panel`.

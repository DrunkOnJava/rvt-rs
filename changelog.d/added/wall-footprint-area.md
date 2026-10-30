- **A wall's `GrossFootprintArea` (#1046, RE-292).** A wall nothing joins
  to or cuts, whose body is its record's box cut back at its joins and that
  is more than twice as long as it is thick, carries `GrossFootprintArea`,
  its length times its width, beside the side areas and volumes it already
  gets. Revit's export writes it on the walls it draws as an extrusion, and
  it is Revit's on all 48 such walls of the BIM4LCA office as built, 47 at
  building permit and 91 of the R2025 models', none different and none that
  Revit's export leaves out (the one wall as long as it is thick, which it
  gives none, is not written): the office goes from 28,065 to 28,113 of the
  39,388 quantities equal as built (Measure run 41146199810). Walls joined to roofs
  or floors get one from Revit on 78 of 85 as built and nothing in the
  record tells which, so they stay unwritten.

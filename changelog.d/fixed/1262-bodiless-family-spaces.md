- **A family instance written as a space is left out where its family draws
  no model form for its parameters.** Revit's export writes a family
  instance that is not a door or a window only when it has a 3D body. On the
  BIM4LCA terrain office, 92 of the 94 "Parking Space" instances whose
  "Export to IFC As" is `IfcSpace` are bicycle bays whose "Stripe Y/N" is 0,
  so no form of their family is drawn, and Revit's export holds only the two
  car bays; `rvt` wrote 89 of the bicycle bays as `IfcSpace` too. The terrain
  office's `IfcSpace` are now the 9 of Revit's export (9 of 98 before), and
  the six reference models, the other office files, the R2025 and the
  held-out models export byte for byte as before. Only instances written as
  `IfcSpace` are tested, other categories are not, and the export
  diagnostics count what is left out (#1262).

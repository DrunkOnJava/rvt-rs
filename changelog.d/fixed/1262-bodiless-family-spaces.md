- **A family instance written as a space is left out where its family draws
  no model form for its parameters.** Revit's export writes a family
  instance that is not a door or a window only when it has a 3D body. On the
  BIM4LCA terrain office, 92 of the 94 "Parking Space" instances whose
  "Export to IFC As" is `IfcSpace` are bicycle bays whose "Stripe Y/NA" is 0,
  so no form of their family is drawn, and Revit's export holds only the two
  car bays. They were written as `IfcSpace` too (#1262).
